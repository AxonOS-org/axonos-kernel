//! Capability-Based Application Isolation
//!
//! Theorem 8.3 (Structural Data Minimisation):
//! Under the AxonOS manifest system, for any application A with manifest M ⊆ K,
//! no event of a prohibited type can be delivered to A.
//!
//! ## Threat Model
//!
//! - A1: Malicious installed application attempting to exfiltrate raw EEG
//! - A2: Compromised application exploiting memory-safety vulnerability
//! - A3: Network adversary intercepting intent observations
//!
//! ## Prohibited Types
//!
//! {RawEEG, ContinuousEmotion, CognitiveProfile, Reidentification}
//!
//! These are absent from the kernel's type catalogue and cannot be added
//! without recompiling the kernel binary.

pub mod manifest;
pub mod catalogue;
pub mod dispatch;

pub use manifest::{Manifest, ManifestBuilder};
pub use catalogue::{Capability, Catalogue};
pub use dispatch::Dispatch;
