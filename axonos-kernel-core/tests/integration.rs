// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <connect@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! Cross-crate integration tests for `axonos-kernel-core`.
//!
//! Unlike the unit tests inside the crate, these compile as an **external
//! consumer**: they reach every type through the crate's public surface —
//! `axonos_kernel_core` and the crates it re-exports (`axonos_scheduler`,
//! `axonos_capability`, `axonos_intent`, `axonos_spsc`, `axonos_time`) — and
//! so they verify two things at once: that the assembled kernel behaves
//! correctly across the seams between its five foundational crates, and that
//! its public API is usable from outside without reaching into internals.
//!
//! Coverage here is deliberately complementary to the in-crate unit tests. It
//! concentrates on the seams those tests do not exercise — the scheduling
//! decision (`schedule_tick`), the clock session-envelope guard, a multi-step
//! observation sequence, and the reachability of the constrained-deadline
//! feasibility feature through the public surface.

use axonos_kernel_core::axonos_capability::{Capability, CapabilitySet, Manifest};
use axonos_kernel_core::axonos_intent::{
    Confidence, Direction, IntentObservation, Kind, NavigationDirection,
};
use axonos_kernel_core::axonos_scheduler::{
    processor_demand_feasible, Feasibility, Instant as SchedInstant, Micros, Task, TaskId,
    TaskInstance, TaskSet,
};
use axonos_kernel_core::axonos_time::{Instant, MockClock};
use axonos_kernel_core::{new_ipc_channel, BciKernel, KernelConfig, KernelInitError, TickError};

/// The reference five-task BCI pipeline, assembled through the public API as
/// an external consumer would. Holds the `Navigation` and `SessionQuality`
/// capabilities.
fn reference_kernel() -> BciKernel<MockClock, 8, 64> {
    let mut config: KernelConfig<8, 64> = KernelConfig::new();
    config
        .add_task(Task::periodic(TaskId(1), Micros(642), Micros(4000)))
        .unwrap();
    config
        .add_task(Task::periodic(TaskId(2), Micros(12), Micros(4000)))
        .unwrap();
    config
        .add_task(Task::periodic(TaskId(3), Micros(18), Micros(4000)))
        .unwrap();
    config
        .add_task(Task::periodic(TaskId(4), Micros(24), Micros(4000)))
        .unwrap();

    let manifest = Manifest::new(
        CapabilitySet::singleton(Capability::Navigation).with(Capability::SessionQuality),
    );

    BciKernel::new(config, manifest, MockClock::new()).expect("reference pipeline must admit")
}

#[test]
fn kernel_assembles_through_public_api() {
    // The whole assembly is reachable from outside the crate, and the
    // admission and response-time results agree with the pipeline's figures.
    let kernel = reference_kernel();
    assert!(kernel.utilisation_scaled() > 170_000);
    assert!(kernel.utilisation_scaled() < 180_000);
    assert_eq!(kernel.response_time_bound(), Micros(696));
    assert!(kernel.manifest().requested.contains(Capability::Navigation));
}

#[test]
fn admission_failure_surfaces_at_construction() {
    // Two tasks at 50% utilisation each: 1.0 total, far above the ceiling.
    let mut config: KernelConfig<4, 32> = KernelConfig::new();
    config
        .add_task(Task::periodic(TaskId(1), Micros(2000), Micros(4000)))
        .unwrap();
    config
        .add_task(Task::periodic(TaskId(2), Micros(2000), Micros(4000)))
        .unwrap();
    let manifest = Manifest::new(CapabilitySet::singleton(Capability::Navigation));
    let result = BciKernel::new(config, manifest, MockClock::new());
    assert!(matches!(result, Err(KernelInitError::Admission(_))));
}

// ── The scheduling decision seam (untested by the in-crate unit tests) ──────

#[test]
fn schedule_tick_selects_earliest_deadline() {
    let mut kernel = reference_kernel();

    // Two ready instances released together; task 2's deadline (period 2000)
    // is earlier than task 1's (period 4000), so EDF must pick task 2.
    let ready = [
        TaskInstance {
            task: Task::periodic(TaskId(1), Micros(100), Micros(4000)),
            released_at: SchedInstant(1000),
        },
        TaskInstance {
            task: Task::periodic(TaskId(2), Micros(100), Micros(2000)),
            released_at: SchedInstant(1000),
        },
    ];

    let picked = kernel
        .schedule_tick(&ready)
        .expect("clock is within envelope");
    assert_eq!(picked, Some(2));
}

#[test]
fn schedule_tick_breaks_ties_by_lower_task_id() {
    let mut kernel = reference_kernel();

    // Equal absolute deadlines: the lower task id wins, deterministically.
    let ready = [
        TaskInstance {
            task: Task::periodic(TaskId(7), Micros(100), Micros(4000)),
            released_at: SchedInstant(1000),
        },
        TaskInstance {
            task: Task::periodic(TaskId(3), Micros(100), Micros(4000)),
            released_at: SchedInstant(1000),
        },
    ];

    assert_eq!(kernel.schedule_tick(&ready).unwrap(), Some(3));
}

#[test]
fn schedule_tick_with_no_ready_tasks_returns_none() {
    let mut kernel = reference_kernel();
    let ready: [TaskInstance; 0] = [];
    assert_eq!(kernel.schedule_tick(&ready).unwrap(), None);
}

#[test]
fn schedule_tick_advances_the_tick_counter() {
    let mut kernel = reference_kernel();
    assert_eq!(kernel.tick_count(), 0);
    let ready: [TaskInstance; 0] = [];
    kernel.schedule_tick(&ready).unwrap();
    kernel.schedule_tick(&ready).unwrap();
    assert_eq!(kernel.tick_count(), 2);
}

// ── The observation pipeline seam: capability gate → intent → IPC ──────────

#[test]
fn observation_sequence_drains_in_order_with_monotonic_sequence() {
    let mut kernel = reference_kernel();
    let ipc = new_ipc_channel::<64>();
    let (mut producer, mut consumer) = ipc.split().unwrap();

    // Produce three observations through the capability-gated path.
    for _ in 0..3 {
        kernel
            .produce_observation(
                &mut producer,
                NavigationDirection::Right,
                Confidence::from_q0_16(0x4000),
            )
            .expect("Navigation is held; the push must succeed");
    }

    // Drain them in FIFO order; sequence numbers must be 1, 2, 3.
    for expected_sequence in 1..=3u32 {
        let bytes = consumer.try_pop().expect("an observation is queued");
        let decoded = IntentObservation::decode(&bytes).expect("we encoded it; it must decode");
        assert_eq!(decoded.kind, Kind::Navigation);
        assert_eq!(
            decoded.direction,
            Direction::Navigation(NavigationDirection::Right)
        );
        assert_eq!(decoded.sequence, expected_sequence);
    }

    // The ring is now empty.
    assert!(consumer.try_pop().is_none());
}

#[test]
fn observation_requires_the_capability_in_the_manifest() {
    // A kernel whose manifest does not hold Navigation must refuse to produce
    // a Navigation observation, before touching the IPC ring.
    let mut config: KernelConfig<8, 64> = KernelConfig::new();
    config
        .add_task(Task::periodic(TaskId(1), Micros(100), Micros(4000)))
        .unwrap();
    let manifest = Manifest::new(CapabilitySet::singleton(Capability::SessionQuality));
    let mut kernel: BciKernel<MockClock, 8, 64> =
        BciKernel::new(config, manifest, MockClock::new()).unwrap();

    let ipc = new_ipc_channel::<64>();
    let (mut producer, _consumer) = ipc.split().unwrap();
    let result =
        kernel.produce_observation(&mut producer, NavigationDirection::Right, Confidence::MIN);
    assert!(matches!(
        result,
        Err(TickError::CapabilityNotInManifest {
            required: Capability::Navigation,
        })
    ));
}

// ── The clock session-envelope guard (untested by the in-crate unit tests) ──

#[test]
fn out_of_envelope_clock_is_rejected_by_both_tick_paths() {
    // A clock whose time lies beyond the session envelope is a configuration
    // error: both the observation path and the scheduling path must refuse it
    // rather than emit a result against an implausible timestamp.
    let mut config: KernelConfig<8, 64> = KernelConfig::new();
    config
        .add_task(Task::periodic(TaskId(1), Micros(100), Micros(4000)))
        .unwrap();
    let manifest = Manifest::new(CapabilitySet::singleton(Capability::Navigation));
    let mut kernel: BciKernel<MockClock, 8, 64> =
        BciKernel::new(config, manifest, MockClock::starting_at(Instant(u64::MAX))).unwrap();

    let ipc = new_ipc_channel::<64>();
    let (mut producer, _consumer) = ipc.split().unwrap();
    let obs = kernel.produce_observation(
        &mut producer,
        NavigationDirection::Right,
        Confidence::from_q0_16(0x4000),
    );
    assert!(matches!(obs, Err(TickError::ClockOutOfEnvelope)));

    let ready: [TaskInstance; 0] = [];
    assert!(matches!(
        kernel.schedule_tick(&ready),
        Err(TickError::ClockOutOfEnvelope)
    ));
}

// ── The 0.3.0 constrained-deadline feature, reached through the kernel ──────

#[test]
fn constrained_deadline_task_composes_through_kernel_construction() {
    // A constrained-deadline task (D < T), built with the 0.3.0 constructor,
    // flows through the kernel config and admits on utilisation as any task
    // does. (The kernel's admission is the Liu–Layland utilisation test; the
    // exact processor-demand check is exercised separately below.)
    let mut config: KernelConfig<8, 64> = KernelConfig::new();
    config
        .add_task(Task::periodic_with_deadline(
            TaskId(1),
            Micros(100),
            Micros(1000),
            Micros(500),
        ))
        .unwrap();
    let manifest = Manifest::new(CapabilitySet::singleton(Capability::Navigation));
    let kernel =
        BciKernel::new(config, manifest, MockClock::new()).expect("U = 0.1 admits comfortably");
    assert!(kernel.utilisation_scaled() > 90_000);
    assert!(kernel.utilisation_scaled() < 110_000);
}

#[test]
fn processor_demand_criterion_is_reachable_through_the_public_surface() {
    // The 0.3.0 feasibility test is part of the kernel's public surface (via
    // the re-exported scheduler). It catches a constrained-deadline set that
    // the utilisation admission test admits: C = 200 within D = 100 needs more
    // work than the deadline allows, though U = 0.2 is well under any ceiling.
    let mut set: TaskSet<4> = TaskSet::new();
    set.push(Task::periodic_with_deadline(
        TaskId(1),
        Micros(200),
        Micros(1000),
        Micros(100),
    ))
    .unwrap();

    // The utilisation admission test is satisfied (U = 0.2 <= 0.25):
    assert!(set.admit(250_000).is_ok());

    // The processor-demand criterion is not, and reports the violating point:
    match processor_demand_feasible(&set) {
        Feasibility::Infeasible { at, demand } => {
            assert_eq!(at, Micros(100));
            assert_eq!(demand, 200);
        }
        other => panic!("expected Infeasible, got {other:?}"),
    }
}
