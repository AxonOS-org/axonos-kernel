//! Dual-Core Real-Time Contract (DC1-DC6)
//!
//! Six-clause contract specifying and bounding inter-processor
//! communication (IPC) between M4F DSP core and Cortex-A53.
//!
//! | ID | Guarantee | Bound | Level |
//! |----|-----------|-------|-------|
//! | DC1 | Pipeline meets deadline every cycle | — | [L2] |
//! | DC2 | SPSC IPC latency bounded | ≤0.2 µs | [L2] |
//! | DC3 | A53 wake-up deterministic | ≤50 µs | [L2] |
//! | DC4 | A53 state memory isolation | N/A | [L1] |
//! | DC5 | Safe-idle on M4F heartbeat loss | ≤12 ms | [L2] |
//! | DC6 | Intent attestation (HMAC-SHA256) | N/A | [L1] |

pub mod dualcore;
pub mod contract;

pub use dualcore::DualCoreContract;
pub use contract::{DcClause, IpcLatency};
