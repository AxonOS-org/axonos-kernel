//! Dual-Core Contract Demo
//!
//! Demonstrates DC1-DC6 contract between M4F and A53.

#![no_std]
#![no_main]

use axonos_kernel::*;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let mut contract = ipc::DualCoreContract::new();
    let mut consent = consent::ConsentFsm::new();

    // Grant consent
    consent.transition(consent::ConsentOp::Grant, 0);

    loop {
        // Send heartbeat every epoch
        contract.send_heartbeat(0);

        // Check A53 is responsive
        if !contract.check_heartbeat(0) {
            // DC5 violation: enter safe-idle
            consent::Interlock::activate_safe_idle();
        }

        // Process intent
        let packet = ipc::IntentPacket {
            class: 1,
            confidence: 200,
            hmac_tag: [0; 4],
            epoch: 0,
            timestamp: 0,
        };

        contract.send_intent(packet).unwrap();

        cortex_m::asm::wfi();
    }
}
