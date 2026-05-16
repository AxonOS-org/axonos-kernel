// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # Kani BMC harnesses for axonos-scheduler
//!
//! Verifies the correctness of admission test, deadline selection, and
//! response-time analysis under bounded model checking.
//!
//! Run with:
//! ```text
//! cargo kani --harness sched_<name>
//! ```

#![cfg_attr(kani, no_std)]

#[cfg(kani)]
use axonos_scheduler::{
    response_time_bound, select_next, Instant, Micros, Task, TaskId, TaskInstance, TaskSet,
};

// ───────────────────────────────────────────────────────────────────────────
// S1: admission test sound under bounded utilisation
// ───────────────────────────────────────────────────────────────────────────

/// **S1.** For any two-task system with `C_i <= T_i`, the admission test
/// passes at `U_max=1.0` if and only if the actual utilisation is `<= 1.0`.
#[cfg(kani)]
#[kani::proof]
fn sched_s1_admission_sound() {
    let c1: u32 = kani::any();
    let t1: u32 = kani::any();
    let c2: u32 = kani::any();
    let t2: u32 = kani::any();

    // Constrain to reasonable BCI-scale parameters.
    kani::assume(t1 > 0 && t1 <= 1_000_000);
    kani::assume(t2 > 0 && t2 <= 1_000_000);
    kani::assume(c1 <= t1);
    kani::assume(c2 <= t2);

    let mut set: TaskSet<4> = TaskSet::new();
    set.push(Task::periodic(TaskId(1), Micros(c1), Micros(t1)))
        .unwrap();
    set.push(Task::periodic(TaskId(2), Micros(c2), Micros(t2)))
        .unwrap();

    let u = set.utilisation_scaled().unwrap();
    let admit_at_one = set.admit(1_000_000); // U_max = 1.0

    // Soundness: admission test passes iff U <= 1.0.
    assert!(admit_at_one.is_ok() == (u <= 1_000_000));
}

// ───────────────────────────────────────────────────────────────────────────
// S2: select_next returns task with smallest absolute deadline
// ───────────────────────────────────────────────────────────────────────────

/// **S2.** For any two ready task instances with distinct deadlines,
/// `select_next` returns the one with the smaller absolute deadline.
#[cfg(kani)]
#[kani::proof]
fn sched_s2_select_picks_earliest_deadline() {
    let id_a: u16 = kani::any();
    let id_b: u16 = kani::any();
    let release_a: u64 = kani::any();
    let release_b: u64 = kani::any();
    let period_a: u32 = kani::any();
    let period_b: u32 = kani::any();

    // Bound parameters to a reasonable range so the solver doesn't blow up
    // on overflow corner cases.
    kani::assume(id_a != id_b);
    // Tight bounds: solver needs only enough to demonstrate ordering, not
    // enumerate the full timer range.
    kani::assume(period_a > 100 && period_a <= 4_000);
    kani::assume(period_b > 100 && period_b <= 4_000);
    kani::assume(release_a <= 10_000);
    kani::assume(release_b <= 10_000);

    let task_a = Task::periodic(TaskId(id_a), Micros(100), Micros(period_a));
    let task_b = Task::periodic(TaskId(id_b), Micros(100), Micros(period_b));

    let inst_a = TaskInstance {
        task: task_a,
        released_at: Instant(release_a),
    };
    let inst_b = TaskInstance {
        task: task_b,
        released_at: Instant(release_b),
    };

    let dl_a = inst_a.absolute_deadline();
    let dl_b = inst_b.absolute_deadline();

    // Only assert when deadlines are distinct; tie-breaking is covered by S3.
    kani::assume(dl_a != dl_b);

    let instances = [inst_a, inst_b];
    let picked = select_next(&instances, Instant(0)).unwrap();

    if dl_a < dl_b {
        assert!(picked.task.id == task_a.id);
    } else {
        assert!(picked.task.id == task_b.id);
    }
}

// ───────────────────────────────────────────────────────────────────────────
// S3: deterministic tie-breaking by TaskId
// ───────────────────────────────────────────────────────────────────────────

/// **S3.** When two ready instances have identical absolute deadlines,
/// `select_next` returns the one with the lower `TaskId`.
#[cfg(kani)]
#[kani::proof]
fn sched_s3_tie_break_by_lower_id() {
    let id_a: u16 = kani::any();
    let id_b: u16 = kani::any();
    kani::assume(id_a < id_b); // a is strictly lower

    let task_a = Task::periodic(TaskId(id_a), Micros(100), Micros(4000));
    let task_b = Task::periodic(TaskId(id_b), Micros(100), Micros(4000));

    let inst_a = TaskInstance {
        task: task_a,
        released_at: Instant(1000),
    };
    let inst_b = TaskInstance {
        task: task_b,
        released_at: Instant(1000),
    };

    // Same release time + same period → same absolute deadline.
    let instances = [inst_a, inst_b];
    let picked = select_next(&instances, Instant(0)).unwrap();

    assert!(picked.task.id == task_a.id, "S3: lower TaskId must win tie");
}

// ───────────────────────────────────────────────────────────────────────────
// S4: response-time bound respects task WCETs (monotonicity)
// ───────────────────────────────────────────────────────────────────────────

/// **S4.** For a single-task set, the response-time bound equals the task's
/// WCET (the busy period equals one execution).
#[cfg(kani)]
#[kani::proof]
fn sched_s4_rta_single_task() {
    let c: u32 = kani::any();
    let t: u32 = kani::any();

    kani::assume(t > 100 && t <= 1_000_000);
    kani::assume(c > 0 && c < t); // U < 1
    kani::assume(c <= t / 2); // keep below threshold so RTA converges in one step

    let mut set: TaskSet<2> = TaskSet::new();
    set.push(Task::periodic(TaskId(1), Micros(c), Micros(t)))
        .unwrap();

    let r = response_time_bound(&set);
    // For a single task with C < T, L_0 = C < T = min(T_j), so R = C.
    assert!(r == Micros(c), "S4: single-task RTA must equal WCET");
}

// ───────────────────────────────────────────────────────────────────────────
// S5: empty task set is trivially schedulable and has zero response time
// ───────────────────────────────────────────────────────────────────────────

/// **S5.** An empty task set passes admission at any ceiling, and its
/// response-time bound is zero.
#[cfg(kani)]
#[kani::proof]
fn sched_s5_empty_set_trivial() {
    let set: TaskSet<4> = TaskSet::new();
    assert!(set.admit(0).is_ok());
    assert!(response_time_bound(&set) == Micros::ZERO);
}

fn main() {
    // This binary exists to host Kani harnesses. Under non-Kani builds,
    // it is inert; under cargo kani, the harness functions below are run.
    #[cfg(not(kani))]
    eprintln!("axonos-scheduler: Kani harness collection. Run with: cargo kani");
}
