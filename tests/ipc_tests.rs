//! IPC Contract Tests
//!
//! Verify DC1-DC6 contract clauses.

use axonos_kernel::ipc::*;

#[test]
fn test_dc2_ipc_latency() {
    // DC2: IPC latency ≤ 0.2 µs [L2]
    let latency = IpcLatency::measured();
    assert!(latency.round_trip_us <= 0.2, "IPC latency {} exceeds 0.2 µs", latency.round_trip_us);
}

#[test]
fn test_dc5_heartbeat_timeout() {
    // DC5: Safe-idle on M4F heartbeat loss ≤ 12 ms
    let mut contract = DualCoreContract::new();

    contract.send_heartbeat(0);

    // Within timeout: OK
    assert!(contract.check_heartbeat(10_000)); // 10 ms

    // Beyond timeout: safe-idle
    assert!(!contract.check_heartbeat(13_000)); // 13 ms
    assert!(contract.is_safe_idle());
}

#[test]
fn test_intent_packet_roundtrip() {
    let contract = DualCoreContract::new();

    let packet = IntentPacket {
        class: 1,
        confidence: 200,
        hmac_tag: [0xAB; 4],
        epoch: 42,
        timestamp: 1000,
    };

    contract.send_intent(packet).unwrap();
    let received = contract.receive_intent().unwrap();

    assert_eq!(received.class, packet.class);
    assert_eq!(received.epoch, packet.epoch);
}
