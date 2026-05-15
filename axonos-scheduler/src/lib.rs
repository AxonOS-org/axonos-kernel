//! # axonos-scheduler
//!
//! Earliest-deadline-first (EDF) scheduling decision logic for AxonOS.
//!
//! ## Scope
//!
//! This crate is the **pure-Rust algorithmic core** of an EDF scheduler. It
//! contains:
//!
//! - The static task representation [`Task`] and [`TaskSet`].
//! - The Liu–Layland admission test ([`TaskSet::utilisation`],
//!   [`TaskSet::admit`]).
//! - The synchronous busy-period response-time analysis (RTA)
//!   ([`response_time_bound`]).
//! - The scheduling decision function [`select_next`], which given a set of
//!   ready tasks and the current time, returns the task with the earliest
//!   absolute deadline.
//!
//! ## What this crate does NOT contain
//!
//! - Context switching (architecture-specific; needs `cortex-m` or `riscv` crate).
//! - Interrupt handlers (target-specific).
//! - Timer driver.
//! - Boot code.
//! - Memory protection / MPU setup.
//! - Stack management.
//!
//! Those are concerns for the full AxonOS kernel which wraps this crate.
//! This crate exists so the scheduling *decisions* can be reasoned about,
//! tested, and (eventually) formally verified independently of any specific
//! hardware platform.
//!
//! ## Mathematical model
//!
//! A periodic task `tau_i = (C_i, T_i, D_i)` has WCET `C_i`, period `T_i`,
//! and relative deadline `D_i`. The current implementation requires
//! `D_i = T_i` for all tasks (implicit-deadline task system), which is the
//! standard case in the BCI signal pipeline. Constrained-deadline scheduling
//! (`D_i < T_i`) is future work.
//!
//! The Liu–Layland EDF feasibility test for implicit-deadline task systems
//! on a uniprocessor: a task set is schedulable under EDF if and only if
//! the total utilisation `U = sum(C_i / T_i) <= 1`. We additionally enforce
//! a user-supplied `U_max` (typically 0.25 for the BCI signal pipeline)
//! for operational margin.
//!
//! The synchronous busy-period equation for EDF with implicit deadlines:
//!
//! ```text
//! L_{k+1} = sum_j ceil(L_k / T_j) * C_j
//! ```
//!
//! converging to a fixed point `L = R` which is the response-time bound.
//!
//! ## References
//!
//! - Liu, C. L. and Layland, J. W. "Scheduling Algorithms for Multiprogramming
//!   in a Hard-Real-Time Environment." JACM 20(1), 1973.
//! - Buttazzo, G. "Hard Real-Time Computing Systems: Predictable Scheduling
//!   Algorithms and Applications." 3rd ed., Springer, 2011.
//! - Baruah, S. K. "Dynamic- and Static-priority Scheduling of Recurring
//!   Real-time Tasks." Real-Time Systems 24(1), 2003.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use core::cmp::Ordering;

// ───────────────────────────────────────────────────────────────────────────
// Time abstraction
// ───────────────────────────────────────────────────────────────────────────

/// Microsecond duration. Holds a non-negative integer.
///
/// The choice of `u32` reflects the embedded target: 32-bit timer counters
/// are nearly universal on Cortex-M, and `u32::MAX` microseconds is approx
/// 71 minutes, which exceeds the longest BCI deadline by orders of magnitude.
/// For absolute time across longer horizons, use [`Instant`] (u64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Micros(pub u32);

impl Micros {
    /// Zero duration.
    pub const ZERO: Self = Self(0);

    /// Maximum representable duration (≈ 71 min).
    pub const MAX: Self = Self(u32::MAX);
}

/// Monotonically increasing absolute time. 64-bit microsecond counter.
///
/// At 1 µs resolution this counter wraps once per ≈ 584 000 years.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(pub u64);

impl Instant {
    /// Origin (boot time / session start).
    pub const ZERO: Self = Self(0);

    /// Returns this instant plus a duration. Saturates on overflow.
    #[must_use]
    pub const fn add_micros(self, d: Micros) -> Self {
        Self(self.0.saturating_add(d.0 as u64))
    }

    /// Returns the duration since `earlier`, saturating at zero if
    /// `earlier` is in the future of `self`.
    #[must_use]
    pub const fn saturating_since(self, earlier: Self) -> Micros {
        let diff = self.0.saturating_sub(earlier.0);
        if diff > u32::MAX as u64 {
            Micros::MAX
        } else {
            Micros(diff as u32)
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Task representation
// ───────────────────────────────────────────────────────────────────────────

/// A periodic real-time task descriptor.
///
/// Tasks are statically declared at compile time; no dynamic allocation.
#[derive(Debug, Clone, Copy)]
pub struct Task {
    /// Worst-case execution time (microseconds).
    pub wcet: Micros,
    /// Period (microseconds). Must be > 0.
    pub period: Micros,
    /// Relative deadline (microseconds). Implicit-deadline systems require
    /// `deadline == period`; explicit-deadline scheduling (`deadline <
    /// period`) is not yet supported in this version.
    pub deadline: Micros,
    /// Stable identifier for the task. Used for diagnostics, audit logs,
    /// and tie-breaking in the scheduler.
    pub id: TaskId,
}

/// Stable per-task identifier. Distinct from any pointer or memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u16);

impl Task {
    /// Construct a task with implicit deadline (`deadline == period`).
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `period.0 == 0` or `wcet.0 > period.0`.
    /// Production code is expected to validate at compile time via
    /// [`TaskSet::admit`].
    #[must_use]
    pub const fn periodic(id: TaskId, wcet: Micros, period: Micros) -> Self {
        debug_assert!(period.0 > 0, "period must be > 0");
        debug_assert!(wcet.0 <= period.0, "wcet cannot exceed period (U > 1)");
        Self {
            wcet,
            period,
            deadline: period,
            id,
        }
    }
}

/// A live task instance: a task descriptor plus its current activation's
/// release time and absolute deadline.
#[derive(Debug, Clone, Copy)]
pub struct TaskInstance {
    /// The static task descriptor.
    pub task: Task,
    /// Wall-clock release time of this activation (i.e., when the period
    /// boundary occurred and the task became ready).
    pub released_at: Instant,
}

impl TaskInstance {
    /// Returns the absolute deadline of this activation.
    #[must_use]
    pub const fn absolute_deadline(&self) -> Instant {
        self.released_at.add_micros(self.task.deadline)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Task set and admission test
// ───────────────────────────────────────────────────────────────────────────

/// A static task set of capacity `N`. Tasks are added in `const`-feasible
/// builder style; the resulting set is immutable.
#[derive(Debug)]
pub struct TaskSet<const N: usize> {
    tasks: [Option<Task>; N],
    count: usize,
}

impl<const N: usize> TaskSet<N> {
    /// Construct an empty task set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tasks: [None; N],
            count: 0,
        }
    }

    /// Push a task into the set. Returns `Err(Full)` if the set is full.
    pub fn push(&mut self, task: Task) -> Result<(), TaskSetFull> {
        if self.count >= N {
            return Err(TaskSetFull);
        }
        self.tasks[self.count] = Some(task);
        self.count += 1;
        Ok(())
    }

    /// Returns the number of tasks in the set.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the total utilisation `U = sum(C_i / T_i)` as a fixed-point
    /// fraction scaled by `SCALE = 1_000_000`.
    ///
    /// The fixed-point representation avoids floating-point in the kernel
    /// (per the AxonOS principle of no FPU dependence on the hot path).
    /// Divide by `SCALE` to recover the rational value.
    ///
    /// Returns `None` on overflow (impossible for well-formed task sets,
    /// but checked for soundness).
    #[must_use]
    pub fn utilisation_scaled(&self) -> Option<u64> {
        const SCALE: u64 = 1_000_000;
        let mut acc: u64 = 0;
        for task in self.tasks.iter().take(self.count).flatten() {
            let wcet = u64::from(task.wcet.0);
            let period = u64::from(task.period.0);
            if period == 0 {
                return None;
            }
            let contrib = wcet.checked_mul(SCALE)?.checked_div(period)?;
            acc = acc.checked_add(contrib)?;
        }
        Some(acc)
    }

    /// Returns the fixed-point scaling factor for [`Self::utilisation_scaled`].
    #[must_use]
    pub const fn utilisation_scale() -> u64 {
        1_000_000
    }

    /// Apply the Liu–Layland admission test with operational ceiling
    /// `u_max_scaled` (in units of `1/SCALE`).
    ///
    /// Returns `Ok(())` if `U <= u_max_scaled`, else `Err(AdmissionFailure)`.
    ///
    /// For the AxonOS BCI pipeline, `u_max_scaled = 250_000` (i.e., 0.25).
    pub fn admit(&self, u_max_scaled: u64) -> Result<(), AdmissionFailure> {
        let u = self
            .utilisation_scaled()
            .ok_or(AdmissionFailure::Overflow)?;
        if u <= u_max_scaled {
            Ok(())
        } else {
            Err(AdmissionFailure::UtilisationExceeded {
                observed: u,
                ceiling: u_max_scaled,
            })
        }
    }

    /// Iterate over the tasks in the set.
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().take(self.count).filter_map(|t| t.as_ref())
    }
}

impl<const N: usize> Default for TaskSet<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Returned by [`TaskSet::push`] when the static capacity is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSetFull;

/// Returned by [`TaskSet::admit`] when the task set fails the EDF
/// feasibility test or fixed-point arithmetic overflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionFailure {
    /// Total utilisation exceeds the configured ceiling.
    UtilisationExceeded {
        /// Observed utilisation in units of 1/SCALE.
        observed: u64,
        /// Configured ceiling in units of 1/SCALE.
        ceiling: u64,
    },
    /// Fixed-point arithmetic overflow during utilisation summation.
    /// Not reachable for well-formed task sets with realistic parameters.
    Overflow,
}

// ───────────────────────────────────────────────────────────────────────────
// Scheduling decision: earliest-deadline-first
// ───────────────────────────────────────────────────────────────────────────

/// Given a slice of ready task instances and the current time, return a
/// reference to the instance with the earliest absolute deadline.
///
/// Returns `None` if `ready` is empty.
///
/// Tie-breaking: when two instances have the same absolute deadline, the
/// one with the lower [`TaskId`] is preferred. This is deterministic and
/// independent of slice ordering, which matters for reproducibility of
/// scheduling traces.
///
/// This function is the central scheduling decision. It is pure: it has no
/// side effects, no internal state, and is deterministic given its inputs.
#[must_use]
pub fn select_next<'a>(ready: &'a [TaskInstance], _now: Instant) -> Option<&'a TaskInstance> {
    ready.iter().min_by(|a, b| {
        match a.absolute_deadline().cmp(&b.absolute_deadline()) {
            Ordering::Equal => a.task.id.cmp(&b.task.id),
            other => other,
        }
    })
}

// ───────────────────────────────────────────────────────────────────────────
// Response-time analysis (synchronous busy period)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the synchronous busy-period response-time bound for the given
/// task set.
///
/// For implicit-deadline EDF on a uniprocessor, if the initial estimate
/// `L_0 = sum(C_i)` is less than `min(T_j)`, then the iteration converges
/// immediately to `L_0` (each ceiling term is exactly 1). For BCI workloads
/// where tasks share a common period and `C_i << T_i`, this is the typical
/// case.
///
/// Returns the bound in microseconds.
///
/// # Panics
///
/// Does not panic on overflow; returns `Micros::MAX` instead, which is the
/// safe (pessimistic) outcome.
#[must_use]
pub fn response_time_bound<const N: usize>(set: &TaskSet<N>) -> Micros {
    if set.is_empty() {
        return Micros::ZERO;
    }

    // Compute L_0 = sum(C_i).
    let mut l: u64 = 0;
    let mut min_period: u32 = u32::MAX;
    for task in set.iter() {
        l = l.saturating_add(u64::from(task.wcet.0));
        if task.period.0 < min_period {
            min_period = task.period.0;
        }
    }

    // If sum(C_i) < min(T_j), each ceiling term in the busy-period equation
    // equals 1 and L_0 is already the fixed point.
    if l < u64::from(min_period) {
        return Micros(u32::try_from(l).unwrap_or(u32::MAX));
    }

    // Otherwise, iterate the busy-period equation until convergence or
    // bound exhaustion. We cap iterations at 64 (well above any realistic
    // pipeline) to ensure termination.
    let mut prev = l;
    for _ in 0..64 {
        let mut next: u64 = 0;
        for task in set.iter() {
            // ceil(prev / T_j) * C_j
            let period = u64::from(task.period.0);
            let wcet = u64::from(task.wcet.0);
            if period == 0 {
                return Micros::MAX;
            }
            // ceil-div without floats: (a + b - 1) / b
            let ceiling = prev.saturating_add(period - 1) / period;
            next = next.saturating_add(ceiling.saturating_mul(wcet));
        }
        if next == prev {
            return Micros(u32::try_from(next).unwrap_or(u32::MAX));
        }
        prev = next;
    }
    // Did not converge within bound. Return MAX as pessimistic upper bound.
    Micros::MAX
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn axonos_pipeline() -> TaskSet<8> {
        // The five-task BCI pipeline from the preprint, with parameters from
        // the sound WCET analysis.
        let mut set: TaskSet<8> = TaskSet::new();
        set.push(Task::periodic(TaskId(1), Micros(642), Micros(4000)))
            .unwrap(); // signal pipeline
        set.push(Task::periodic(TaskId(2), Micros(12), Micros(4000)))
            .unwrap(); // consent FSM
        set.push(Task::periodic(TaskId(3), Micros(18), Micros(4000)))
            .unwrap(); // HMAC attestation
        set.push(Task::periodic(TaskId(4), Micros(24), Micros(4000)))
            .unwrap(); // BLE intent egress
        set.push(Task::periodic(TaskId(5), Micros(100), Micros(1_000_000)))
            .unwrap(); // diagnostics
        set
    }

    #[test]
    fn empty_set_utilisation_is_zero() {
        let s: TaskSet<4> = TaskSet::new();
        assert_eq!(s.utilisation_scaled(), Some(0));
    }

    #[test]
    fn axonos_pipeline_utilisation_under_25pct() {
        let s = axonos_pipeline();
        let u = s.utilisation_scaled().unwrap();
        // Expected: 642/4000 + 12/4000 + 18/4000 + 24/4000 + 100/1000000
        //         = (642+12+18+24)/4000 + 100/1_000_000
        //         = 696/4000 + 0.0001
        //         ≈ 0.174 + 0.0001 = 0.1741
        // Scaled by 1_000_000: ≈ 174_100
        assert!(
            u < 250_000,
            "utilisation must be below U_max=0.25; got {} / 1_000_000",
            u
        );
        assert!(u > 170_000, "sanity check: u should be ≈ 0.174");
    }

    #[test]
    fn axonos_pipeline_admits_at_u_max_025() {
        let s = axonos_pipeline();
        assert_eq!(s.admit(250_000), Ok(()));
    }

    #[test]
    fn high_utilisation_rejected() {
        // Two tasks totalling 80% utilisation, ceiling 25%.
        let mut s: TaskSet<4> = TaskSet::new();
        s.push(Task::periodic(TaskId(1), Micros(2000), Micros(4000)))
            .unwrap(); // 0.5
        s.push(Task::periodic(TaskId(2), Micros(1200), Micros(4000)))
            .unwrap(); // 0.3
        match s.admit(250_000) {
            Err(AdmissionFailure::UtilisationExceeded { observed, ceiling }) => {
                assert!(observed > ceiling);
                assert_eq!(ceiling, 250_000);
            }
            other => panic!("expected UtilisationExceeded, got {other:?}"),
        }
    }

    #[test]
    fn select_next_picks_earliest_deadline() {
        let task_a = Task::periodic(TaskId(1), Micros(100), Micros(4000));
        let task_b = Task::periodic(TaskId(2), Micros(100), Micros(2000));

        let instances = [
            TaskInstance {
                task: task_a,
                released_at: Instant(1000),
            },
            TaskInstance {
                task: task_b,
                released_at: Instant(1000),
            },
        ];

        let picked = select_next(&instances, Instant(1500)).unwrap();
        // Task B has deadline 1000 + 2000 = 3000; Task A has 1000 + 4000 = 5000.
        assert_eq!(picked.task.id, TaskId(2));
    }

    #[test]
    fn select_next_tie_breaks_by_lower_task_id() {
        let task_a = Task::periodic(TaskId(7), Micros(100), Micros(4000));
        let task_b = Task::periodic(TaskId(3), Micros(100), Micros(4000));

        let instances = [
            TaskInstance {
                task: task_a,
                released_at: Instant(1000),
            },
            TaskInstance {
                task: task_b,
                released_at: Instant(1000),
            },
        ];

        let picked = select_next(&instances, Instant(1500)).unwrap();
        // Same deadline (5000); tie-break to lower id.
        assert_eq!(picked.task.id, TaskId(3));
    }

    #[test]
    fn select_next_empty_returns_none() {
        let instances: [TaskInstance; 0] = [];
        assert!(select_next(&instances, Instant(0)).is_none());
    }

    #[test]
    fn response_time_bound_axonos_pipeline() {
        let s = axonos_pipeline();
        let r = response_time_bound(&s);
        // L_0 = 642 + 12 + 18 + 24 + 100 = 796.
        // min(T_j) = 4000. L_0 < min(T_j), so R = 796.
        assert_eq!(r, Micros(796));
    }

    #[test]
    fn response_time_bound_empty_set_is_zero() {
        let s: TaskSet<4> = TaskSet::new();
        assert_eq!(response_time_bound(&s), Micros::ZERO);
    }

    #[test]
    fn task_set_full_rejects_overflow() {
        let mut s: TaskSet<2> = TaskSet::new();
        s.push(Task::periodic(TaskId(1), Micros(100), Micros(1000)))
            .unwrap();
        s.push(Task::periodic(TaskId(2), Micros(100), Micros(1000)))
            .unwrap();
        assert_eq!(
            s.push(Task::periodic(TaskId(3), Micros(100), Micros(1000))),
            Err(TaskSetFull)
        );
    }
}
