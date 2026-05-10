//! Consent Demo (TrustZone)
//!
//! Demonstrates consent FSM in Cortex-M33 Secure World.

#![no_std]
#![no_main]

use axonos_kernel::*;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    // Initialize TrustZone
    platform::CortexM33::init();
    platform::CortexM33::enter_secure();

    let mut fsm = consent::ConsentFsm::new();
    let mut interlock = consent::Interlock::new(0);

    // User grants consent
    fsm.transition(consent::ConsentOp::Grant, 0);
    interlock.update(&fsm, true);

    assert!(interlock.state() == consent::InterlockState::Active);

    // User withdraws consent
    fsm.transition(consent::ConsentOp::Withdraw, 1000);
    interlock.update(&fsm, true);

    assert!(interlock.state() == consent::InterlockState::SafeIdle);

    loop {
        cortex_m::asm::wfi();
    }
}
