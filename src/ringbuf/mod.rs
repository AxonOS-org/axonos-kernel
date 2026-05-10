//! Zero-Copy Ring Buffer
//!
//! Single-producer single-consumer (SPSC) ring buffer with Vyukov
//! sequence-number protocol and Release-Acquire memory ordering.
//!
//! ## Theorem 6.3 (SPSC Sequence-Number Correctness)
//!
//! Under the Rust/C++11 memory model, the consumer observes the
//! producer's payload exactly as written, on any hardware implementing
//! the model (including ARMv7-M and ARMv8-A).
//!
//! ## Safety
//!
//! `#![forbid(unsafe_code)]` applies to all modules except the SPSC
//! payload read/write path, which uses two targeted unsafe blocks
//! formally justified by Theorem 6.3 and bounded-model-checked via Kani.

pub mod spsc;
pub mod sequence;

pub use spsc::SpscRingBuffer;
pub use sequence::SequenceNumber;
