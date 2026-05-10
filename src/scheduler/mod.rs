//! # EDF Scheduler Module
//!
//! Earliest-Deadline-First scheduler with Liu-Layland schedulability test.
//! Conservative admission ceiling U_max = 0.25.
//!
//! ## Theorem 5.2 (Liu-Layland EDF)
//! A set of n periodic tasks with D_i = T_i is schedulable on a uniprocessor
//! under EDF if and only if U ≤ 1.
//!
//! ## Proposition 5.4 (Admission Ceiling)
//! The ceiling U_max = 0.25 is satisfied by the binding L2-inferred utilisation
//! U^L2 = 0.2181. Headroom: 0.0319.

pub mod edf;
pub mod task;
pub mod admission;

pub use edf::EdfScheduler;
pub use task::{Task, TaskId, Deadline, Period, Wcet, Priority};
pub use admission::{AdmissionController, AdmissionResult};
