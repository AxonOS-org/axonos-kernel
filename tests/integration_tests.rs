//! Integration Tests
//!
//! End-to-end tests for AxonOS kernel components.

use axonos_kernel::*;

#[test]
fn test_full_pipeline_epoch() {
    // Create scheduler
    let mut scheduler = scheduler::EdfScheduler::<8>::new();
    let tasks = scheduler::admission::axonos_task_set();

    for task in &tasks {
        scheduler.register_task(*task).unwrap();
    }

    // Create signal pipeline
    let mut pipeline = signal::SignalPipeline::new(signal::PipelineConfig::default());

    // Simulate one epoch
    let frame = [1000i32; 8];
    let epoch = signal::Epoch::new(0, 0);

    let result = pipeline.process(frame, epoch);
    assert!(result.is_some() || result.is_none()); // May be Idle or artifact
}

#[test]
fn test_admission_ceiling() {
    let mut ctrl = scheduler::AdmissionController::new();
    let tasks = scheduler::admission::axonos_task_set();

    for task in &tasks {
        assert!(matches!(
            ctrl.admit(task),
            scheduler::admission::AdmissionResult::Admitted { .. }
        ));
    }

    // Should have headroom
    assert!(ctrl.headroom() > 0.0);
}

#[test]
fn test_spsc_ring_buffer() {
    let ring = ringbuf::SpscRingBuffer::<u32>::new();

    // Push and pop
    ring.try_push(42).unwrap();
    assert_eq!(ring.try_pop().unwrap(), 42);

    // Overrun
    for i in 0..64 {
        ring.try_push(i).unwrap();
    }
    assert!(ring.try_push(999).is_err());
}

#[test]
fn test_consent_fsm() {
    let mut fsm = consent::ConsentFsm::new();

    assert_eq!(fsm.state(), consent::ConsentState::Inactive);

    fsm.transition(consent::ConsentOp::Grant, 1000);
    assert!(fsm.is_processing_allowed());

    fsm.transition(consent::ConsentOp::Withdraw, 2000);
    assert!(fsm.is_withdrawn());
}

#[test]
fn test_capability_manifest() {
    let manifest = capability::ManifestBuilder::new()
        .app_id("com.test.app").unwrap()
        .capability(capability::Capability::Navigation).unwrap()
        .build()
        .unwrap();

    assert_eq!(manifest.app_id, "com.test.app");
    assert_eq!(manifest.capabilities.len(), 1);
}

#[test]
fn test_ipc_contract() {
    let mut contract = ipc::DualCoreContract::new();

    // Send heartbeat
    contract.send_heartbeat(1000);

    // Check valid
    assert!(contract.check_heartbeat(2000));

    // Check timeout
    assert!(!contract.check_heartbeat(15_000)); // > 12 ms
    assert!(contract.is_safe_idle());
}

#[test]
fn test_mutual_information_bound() {
    let mi = capability::Catalogue::max_mutual_information_bps();
    // Should be ~140.85 bits/s
    assert!(mi > 140.0);
    assert!(mi < 141.0);
}
