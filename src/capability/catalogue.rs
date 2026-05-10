//! Capability Catalogue
//!
//! Definition 8.1: A capability κ = (T, r) is a typed permission token
//! where T ∈ T is the event type and r ∈ R+ is the maximum delivery rate (Hz).
//!
//! Definition 8.2: A type T ∈ T is prohibited if T ∉ π₁(K).
//!
//! Table 9: Permitted capability catalogue v1

/// Capability types (permitted)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Navigation: {Left, Right, Up, Down, Idle} @ 50 Hz
    Navigation,
    /// WorkloadAdvisory: {Low, Medium, High} @ 1 Hz
    WorkloadAdvisory,
    /// SessionQuality: {Good, Degraded, Lost} @ 2 Hz
    SessionQuality,
    /// ArtifactEvents: {Eye, Muscle, Motion, Electrode} @ 10 Hz
    ArtifactEvents,
}

/// Prohibited capability types (never in catalogue)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProhibitedCapability {
    /// Raw EEG samples (never crosses application boundary)
    RawEeg,
    /// Continuous emotion inference
    ContinuousEmotion,
    /// Cognitive profile read
    CognitiveProfile,
    /// Re-identification
    Reidentification,
}

impl Capability {
    /// Maximum delivery rate [Hz]
    pub fn max_rate_hz(&self) -> u32 {
        match self {
            Self::Navigation => 50,
            Self::WorkloadAdvisory => 1,
            Self::SessionQuality => 2,
            Self::ArtifactEvents => 10,
        }
    }

    /// Payload alphabet cardinality
    pub fn payload_cardinality(&self) -> usize {
        match self {
            Self::Navigation => 5,      // Left, Right, Up, Down, Idle
            Self::WorkloadAdvisory => 3, // Low, Medium, High
            Self::SessionQuality => 3,   // Good, Degraded, Lost
            Self::ArtifactEvents => 4,   // Eye, Muscle, Motion, Electrode
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::WorkloadAdvisory => "WorkloadAdvisory",
            Self::SessionQuality => "SessionQuality",
            Self::ArtifactEvents => "ArtifactEvents",
        }
    }

    /// All permitted capabilities
    pub fn all() -> [Capability; 4] {
        [
            Capability::Navigation,
            Capability::WorkloadAdvisory,
            Capability::SessionQuality,
            Capability::ArtifactEvents,
        ]
    }
}

/// Capability catalogue (static, kernel-defined)
pub struct Catalogue;

impl Catalogue {
    /// Check if capability is in catalogue
    pub fn contains(cap: &Capability) -> bool {
        matches!(cap, 
            Capability::Navigation |
            Capability::WorkloadAdvisory |
            Capability::SessionQuality |
            Capability::ArtifactEvents
        )
    }

    /// Check if type is prohibited
    pub fn is_prohibited(_cap: &ProhibitedCapability) -> bool {
        true // All prohibited types are permanently absent
    }

    /// Maximum mutual information rate [bits/s]
    ///
    /// Theorem 9.1: I(X; Y) ≤ H(Y) ≤ Σ_κ r_κ · log₂|P(κ)|
    ///
    /// For default manifest with all 4 capabilities:
    /// 50·log₂5 + 1·log₂3 + 2·log₂3 + 10·log₂4 = 140.85 bits/s
    pub fn max_mutual_information_bps() -> f32 {
        Capability::all().iter()
            .map(|c| {
                let rate = c.max_rate_hz() as f32;
                let card = c.payload_cardinality() as f32;
                rate * card.log2()
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutual_information_bound() {
        let mi = Catalogue::max_mutual_information_bps();
        // 50*log2(5) + 1*log2(3) + 2*log2(3) + 10*log2(4)
        // = 50*2.322 + 1*1.585 + 2*1.585 + 10*2.0
        // = 116.1 + 1.585 + 3.17 + 20 = 140.855
        assert!((mi - 140.85).abs() < 0.1);
    }

    #[test]
    fn test_prohibited_types() {
        assert!(Catalogue::is_prohibited(&ProhibitedCapability::RawEeg));
        assert!(Catalogue::is_prohibited(&ProhibitedCapability::ContinuousEmotion));
    }
}
