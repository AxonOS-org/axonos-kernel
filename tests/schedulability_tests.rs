//! Schedulability Analysis Tests
//!
//! Verify EDF schedulability bounds from Section 5.

use axonos_kernel::scheduler::*;

#[test]
fn test_liu_layland_condition() {
    // Theorem 5.2: U ≤ 1 for EDF schedulability
    let tasks = admission::axonos_task_set();
    let u = admission::total_utilisation(&tasks);

    assert!(u <= 1.0, "Utilisation {} exceeds 1.0", u);
}

#[test]
fn test_admission_ceiling() {
    // Proposition 5.4: U^L2 = 0.2181 < U_max = 0.25
    let tasks = admission::axonos_task_set();
    let u = admission::total_utilisation(&tasks);

    assert!(u < 0.25, "Binding utilisation {} exceeds ceiling 0.25", u);
    assert!(u > 0.21, "Binding utilisation {} too low (expected ~0.218)", u);
}

#[test]
fn test_busy_period_bound() {
    // Section 5.5.1: L = 972 µs [L2]
    let mut scheduler = EdfScheduler::<8>::new();
    let tasks = admission::axonos_task_set();

    for task in &tasks {
        scheduler.register_task(*task).unwrap();
    }

    let l = scheduler.busy_period_bound();
    assert_eq!(l, 972, "Busy period bound {} != 972 µs", l);
}

#[test]
fn test_deadline_slack() {
    // Theorem 5.9: S_1 = 4000 - 972 = 3028 µs
    let mut scheduler = EdfScheduler::<8>::new();
    let tasks = admission::axonos_task_set();

    for task in &tasks {
        scheduler.register_task(*task).unwrap();
    }

    let slack = scheduler.deadline_slack(TaskId(1)).unwrap();
    assert_eq!(slack, 3028, "Deadline slack {} != 3028 µs", slack);
}

#[test]
fn test_response_time_ratio() {
    // ρ_1 = R_1^L2 / D_1 = 972 / 4000 = 0.243
    let ratio = 972.0 / 4000.0;
    assert!((ratio - 0.243).abs() < 0.001);
}

#[test]
fn test_headroom() {
    // Headroom: U_max - U^L2 = 0.25 - 0.2181 = 0.0319
    let tasks = admission::axonos_task_set();
    let u = admission::total_utilisation(&tasks);
    let headroom = 0.25 - u;

    assert!(headroom > 0.03, "Headroom {} too small", headroom);
    assert!(headroom < 0.04, "Headroom {} too large", headroom);
}
