//! Event Dispatch
//!
//! Theorem 8.3 (Structural Data Minimisation):
//! dispatch(T) = ∅ for all prohibited T.

use super::{Capability, Manifest};
use crate::signal::MotorImageryClass;

/// Event dispatch function
pub struct Dispatch;

/// Intent observation delivered to application
#[derive(Debug, Clone, Copy)]
pub struct IntentObservation {
    pub capability: Capability,
    pub payload: IntentPayload,
    pub confidence: f32,
    pub epoch: u64,
    pub attestation_tag: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentPayload {
    Direction(NavigationDirection),
    Workload(WorkloadLevel),
    Quality(SessionQuality),
    Artifact(ArtifactType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection { Left, Right, Up, Down, Idle }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadLevel { Low, Medium, High }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionQuality { Good, Degraded, Lost }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType { Eye, Muscle, Motion, Electrode }

impl Dispatch {
    pub fn can_dispatch(manifest: &Manifest, cap: &Capability) -> bool {
        manifest.capabilities.iter().any(|(c, _)| c == cap)
    }

    pub fn classify_to_intent(
        class: MotorImageryClass,
        confidence: f32,
        epoch: u64,
    ) -> IntentObservation {
        let (cap, payload) = match class {
            MotorImageryClass::Left => (Capability::Navigation, IntentPayload::Direction(NavigationDirection::Left)),
            MotorImageryClass::Right => (Capability::Navigation, IntentPayload::Direction(NavigationDirection::Right)),
            MotorImageryClass::Feet => (Capability::Navigation, IntentPayload::Direction(NavigationDirection::Down)),
            MotorImageryClass::Tongue => (Capability::Navigation, IntentPayload::Direction(NavigationDirection::Up)),
            MotorImageryClass::Idle => (Capability::Navigation, IntentPayload::Direction(NavigationDirection::Idle)),
        };
        IntentObservation { capability: cap, payload, confidence, epoch, attestation_tag: [0u8; 32] }
    }

    pub fn residual_uncertainty_bits() -> f32 {
        let joint_cardinality = 5.0 * 3.0 * 3.0 * 4.0;
        joint_cardinality.log2()
    }
}
