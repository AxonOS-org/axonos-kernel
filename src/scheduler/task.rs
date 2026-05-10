//! Task model for EDF scheduling
//!
//! Definition 5.1: A periodic task τ_i = (C_i, T_i, D_i, φ_i) has:
//! - worst-case execution time C_i ∈ R+
//! - period T_i ∈ R+
//! - relative deadline D_i ∈ R+
//! - phase offset φ_i ≥ 0

use core::cmp::Ordering;

/// Unique task identifier (1..MAX_TASKS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u8);

/// Worst-case execution time [µs]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wcet(pub u32);

/// Task period [µs]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Period(pub u32);

/// Relative deadline [µs] — AxonOS uses D_i = T_i (deadline-equals-period)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(pub u32);

/// EDF priority is derived from absolute deadline (earlier = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority {
    /// Absolute deadline [µs]
    pub absolute_deadline: u32,
    /// Tie-breaker: lower task ID wins
    pub task_id: TaskId,
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Earlier deadline = higher priority
        self.absolute_deadline
            .cmp(&other.absolute_deadline)
            .reverse() // reverse because min-heap
            .then_with(|| self.task_id.0.cmp(&other.task_id.0).reverse())
    }
}

/// Periodic task definition
#[derive(Debug, Clone, Copy)]
pub struct Task {
    /// Unique identifier
    pub id: TaskId,
    /// Worst-case execution time [µs]
    pub wcet: Wcet,
    /// Period [µs]
    pub period: Period,
    /// Relative deadline [µs] — equals period in AxonOS
    pub deadline: Deadline,
    /// Phase offset [µs]
    pub phase: u32,
    /// Task utilisation: C_i / T_i
    pub utilisation: f32,
    /// Human-readable name
    pub name: &'static str,
}

impl Task {
    /// Create a new task with deadline-equals-period
    ///
    /// # Panics
    /// Panics if period is zero (division by zero in utilisation)
    pub const fn new(id: u8, wcet_us: u32, period_us: u32, name: &'static str) -> Self {
        assert!(period_us > 0, "period must be positive");
        let util = (wcet_us as f32) / (period_us as f32);
        Self {
            id: TaskId(id),
            wcet: Wcet(wcet_us),
            period: Period(period_us),
            deadline: Deadline(period_us), // D_i = T_i
            phase: 0,
            utilisation: util,
            name,
        }
    }

    /// Compute absolute deadline for job k
    pub fn absolute_deadline(&self, job_index: u32) -> u32 {
        self.phase + job_index * self.period.0 + self.deadline.0
    }

    /// Compute absolute release time for job k
    pub fn release_time(&self, job_index: u32) -> u32 {
        self.phase + job_index * self.period.0
    }
}

/// Task state in the scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to execute
    Ready,
    /// Task is currently running
    Running,
    /// Task has completed its current job
    Completed,
    /// Task missed its deadline (safety violation)
    DeadlineMiss,
}

/// Job instance of a periodic task
#[derive(Debug, Clone, Copy)]
pub struct Job {
    /// Parent task
    pub task_id: TaskId,
    /// Job index within the task
    pub job_index: u32,
    /// Release time [µs]
    pub release: u32,
    /// Absolute deadline [µs]
    pub deadline: u32,
    /// Remaining execution time [µs]
    pub remaining: u32,
    /// Current state
    pub state: TaskState,
}

impl Job {
    /// Create a new job instance
    pub fn new(task: &Task, job_index: u32) -> Self {
        Self {
            task_id: task.id,
            job_index,
            release: task.release_time(job_index),
            deadline: task.absolute_deadline(job_index),
            remaining: task.wcet.0,
            state: TaskState::Ready,
        }
    }

    /// Check if job has completed
    pub fn is_complete(&self) -> bool {
        self.remaining == 0
    }

    /// Check if deadline has been missed at time t
    pub fn is_missed(&self, now: u32) -> bool {
        now > self.deadline && self.remaining > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_utilisation() {
        let t = Task::new(1, 640, 4000, "signal_pipeline");
        assert_eq!(t.utilisation, 0.1601);
    }

    #[test]
    fn test_priority_ordering() {
        let p1 = Priority { absolute_deadline: 1000, task_id: TaskId(1) };
        let p2 = Priority { absolute_deadline: 2000, task_id: TaskId(2) };
        assert!(p1 > p2); // earlier deadline = higher priority
    }
}
