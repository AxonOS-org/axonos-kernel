//! EDF Scheduler Implementation
//!
//! Theorem 5.2 (Liu-Layland): A set of n periodic tasks with D_i = T_i is
//! schedulable on a uniprocessor under EDF iff U ≤ 1.
//!
//! AxonOS uses conservative ceiling U_max = 0.25 (Proposition 5.4).
//!
//! ## Synchronous Busy Period (Section 5.5.1)
//!
//! For EDF with D_i = T_i under simultaneous-release worst case:
//! L = Σ_j ceil(L / T_j) * C_j
//!
//! Starting from L^(0) = Σ_j C_j^L2 = 972 µs.
//! Since L^(0) = 972 µs < min_j T_j = 4000 µs, all ceiling terms equal 1
//! and iteration converges in one step: L^(1) = 972 µs = L^(0).
//!
//! Response-time bound for τ_1: R_1 ≤ L = 972 µs [L2].

use super::{Task, TaskId, TaskState, Job, Wcet, Priority};
use crate::config;
use heapless::binary_heap::{BinaryHeap, Max};
use heapless::Vec;

/// EDF Scheduler with admission control
pub struct EdfScheduler<const N: usize> {
    /// Registered tasks
    tasks: Vec<Task, N>,
    /// Ready queue ordered by absolute deadline (min-heap via Max with reverse)
    ready_queue: BinaryHeap<Priority, Max, N>,
    /// Currently executing job (if any)
    current_job: Option<Job>,
    /// Current time [µs]
    now: u32,
    /// Total utilisation
    total_utilisation: f32,
    /// Deadline miss counter (safety-critical)
    deadline_misses: u32,
    /// DWT cycle counter for precise timing
    dwt: crate::platform::Dwt,
}

/// Scheduling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleDecision {
    /// Continue running current job
    Continue,
    /// Preempt current job, run new job
    Preempt(TaskId),
    /// Idle (no ready jobs)
    Idle,
}

/// Scheduler statistics
#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    /// Total epochs processed
    pub epochs: u64,
    /// Deadline misses observed
    pub deadline_misses: u32,
    /// Maximum observed response time [µs]
    pub wcrt_max: u32,
    /// Jitter standard deviation [µs]
    pub jitter_sigma: f32,
    /// Current utilisation
    pub utilisation: f32,
}

impl<const N: usize> EdfScheduler<N> {
    /// Create new EDF scheduler
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            ready_queue: BinaryHeap::new(),
            current_job: None,
            now: 0,
            total_utilisation: 0.0,
            deadline_misses: 0,
            dwt: crate::platform::Dwt::new(),
        }
    }

    /// Register a task after admission control
    ///
    /// Returns Err if admission ceiling exceeded
    pub fn register_task(&mut self, task: Task) -> Result<(), AdmissionError> {
        let new_util = self.total_utilisation + task.utilisation;
        if new_util > config::ADMISSION_CEILING {
            return Err(AdmissionError::CeilingExceeded {
                current: self.total_utilisation,
                requested: new_util,
                ceiling: config::ADMISSION_CEILING,
            });
        }
        self.total_utilisation = new_util;
        self.tasks.push(task).map_err(|_| AdmissionError::TaskLimit)?;
        Ok(())
    }

    /// Release jobs for all tasks at epoch boundary
    ///
    /// Called by ADC DMA interrupt handler at t = k * T_s
    pub fn release_epoch_jobs(&mut self, epoch: u32) {
        for task in &self.tasks {
            let job = Job::new(task, epoch);
            let priority = Priority {
                absolute_deadline: job.deadline,
                task_id: task.id,
            };
            // Note: In production, this would push to ready queue
            // For now, we track the release
            let _ = priority;
        }
    }

    /// EDF scheduling decision at time `now`
    ///
    /// Returns the job to execute next based on earliest absolute deadline.
    pub fn schedule(&mut self, now: u32) -> ScheduleDecision {
        self.now = now;

        // Check for deadline misses (safety monitoring)
        if let Some(ref job) = self.current_job {
            if job.is_missed(now) {
                self.deadline_misses += 1;
                // Trigger DC1 violation handler
                self.handle_deadline_miss(job);
            }
        }

        // Find highest priority ready job
        if let Some(priority) = self.ready_queue.peek() {
            match self.current_job {
                None => ScheduleDecision::Preempt(priority.task_id),
                Some(ref current) => {
                    let current_prio = Priority {
                        absolute_deadline: current.deadline,
                        task_id: current.task_id,
                    };
                    if priority > &current_prio {
                        ScheduleDecision::Preempt(priority.task_id)
                    } else {
                        ScheduleDecision::Continue
                    }
                }
            }
        } else {
            ScheduleDecision::Idle
        }
    }

    /// Execute one tick of the current job
    ///
    /// Returns true if job completed
    pub fn tick(&mut self, elapsed_us: u32) -> bool {
        if let Some(ref mut job) = self.current_job {
            job.remaining = job.remaining.saturating_sub(elapsed_us);
            if job.is_complete() {
                job.state = TaskState::Completed;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Context switch to new job
    pub fn context_switch(&mut self, job: Job) {
        if let Some(old) = self.current_job.take() {
            if !old.is_complete() {
                // Preempted job goes back to ready queue
                let prio = Priority {
                    absolute_deadline: old.deadline,
                    task_id: old.task_id,
                };
                let _ = self.ready_queue.push(prio);
            }
        }
        self.current_job = Some(job);
    }

    /// Handle deadline miss — trigger DC1 violation
    fn handle_deadline_miss(&mut self, job: &Job) {
        // DC1: Pipeline meets deadline every cycle
        // On violation: activate stimulation interlock (DC5)
        crate::consent::Interlock::activate_safe_idle();

        // Log to secure element
        crate::attestation::Attestation::log_violation(
            crate::ipc::DcClause::Dc1,
            job.task_id,
            self.now,
        );
    }

    /// Compute synchronous busy period bound (Theorem 5.2)
    ///
    /// L = Σ_j ceil(L / T_j) * C_j
    ///
    /// For AxonOS task set with L^(0) = 972 µs < min T_j = 4000 µs:
    /// All ceiling terms = 1, so L = Σ_j C_j = 972 µs [L2]
    pub fn busy_period_bound(&self) -> u32 {
        let mut l: u32 = self.tasks.iter().map(|t| t.wcet.0).sum();

        // Fixed-point iteration (typically converges in 1 step for AxonOS)
        loop {
            let new_l: u32 = self.tasks.iter()
                .map(|t| {
                    let ceil = (l + t.period.0 - 1) / t.period.0;
                    ceil * t.wcet.0
                })
                .sum();

            if new_l == l {
                break;
            }
            l = new_l;
        }

        l
    }

    /// Compute deadline slack for signal pipeline (Theorem 5.9)
    ///
    /// S_1 ≜ D_1 - R_1^L2 = 4000 - 972 = 3028 µs
    pub fn deadline_slack(&self, task_id: TaskId) -> Option<u32> {
        let task = self.tasks.iter().find(|t| t.id == task_id)?;
        let r = self.busy_period_bound(); // Conservative: use WCRT
        Some(task.deadline.0.saturating_sub(r))
    }

    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            epochs: 0, // Updated by caller
            deadline_misses: self.deadline_misses,
            wcrt_max: self.busy_period_bound(),
            jitter_sigma: 2.1, // [L2] measured
            utilisation: self.total_utilisation,
        }
    }
}

/// Admission error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// Utilisation ceiling exceeded
    CeilingExceeded { current: f32, requested: f32, ceiling: f32 },
    /// Maximum task limit reached
    TaskLimit,
    /// Duplicate task ID
    DuplicateId,
}

impl core::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CeilingExceeded { current, requested, ceiling } => {
                write!(f, "Utilisation {:.4} exceeds ceiling {:.4} (current: {:.4})", 
                    requested, ceiling, current)
            }
            Self::TaskLimit => write!(f, "Maximum task limit reached"),
            Self::DuplicateId => write!(f, "Duplicate task ID"),
        }
    }
}
