//! Consent State Machine and Stimulation Interlock
//!
//! DC5: Safe-idle on M4F heartbeat loss ≤12 ms [L2]
//!
//! The consent FSM manages user consent states and controls
//! the stimulation interlock.

pub mod fsm;
pub mod interlock;

pub use fsm::{ConsentFsm, ConsentState};
pub use interlock::Interlock;
