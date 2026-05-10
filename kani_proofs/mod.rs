//! Kani Bounded Model Checking Proofs
//!
//! Appendix A: Verification of two safety-critical finite-state properties.
//!
//! ## SPSC Ring Buffer (A.1)
//!
//! File: `kani_proofs/spsc_proof.rs`
//!
//! | Proof | Property | Unwind | Time |
//! |-------|----------|--------|------|
//! | K1 | No data race | 8 | 4.2s |
//! | K2 | Wait-freedom | 4 | 1.1s |
//! | K3 | Memory ordering / payload integrity | 2 | 0.8s |
//!
//! ## Heartbeat FSM (A.2)
//!
//! File: `kani_proofs/heartbeat_proof.rs`
//!
//! | Proof | Property | Unwind | Time |
//! |-------|----------|--------|------|
//! | K1 | Safety | 12 | 2.3s |
//! | K2 | Liveness | 12 | 1.8s |
//! | K3 | Monotonicity | 8 | 0.9s |
//!
//! ## Scope of Verification
//!
//! Kani bounded model checking verifies all reachable states within the unwind bound.
//! The FSM has |S| = 3 states and |E| = 2 events; the diameter is ≤ 4 transitions,
//! so the unwind bound 12 covers 3× the diameter, sufficient for full state-space
//! coverage of liveness properties.

pub mod spsc_proof;
pub mod heartbeat_proof;
