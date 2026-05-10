//! Dual-Core Contract Definitions
//!
//! Table 6: Six-clause real-time contract DC1-DC6

/// Contract clause identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DcClause {
    /// DC1: Pipeline meets deadline every cycle
    Dc1,
    /// DC2: SPSC IPC latency bounded ≤0.2 µs
    Dc2,
    /// DC3: A53 wake-up deterministic ≤50 µs
    Dc3,
    /// DC4: A53 state memory isolation
    Dc4,
    /// DC5: Safe-idle on M4F heartbeat loss ≤12 ms
    Dc5,
    /// DC6: Intent attestation (HMAC-SHA256)
    Dc6,
}

impl DcClause {
    /// Human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Dc1 => "Pipeline meets deadline every cycle",
            Self::Dc2 => "SPSC IPC latency bounded ≤0.2 µs",
            Self::Dc3 => "A53 wake-up deterministic ≤50 µs",
            Self::Dc4 => "A53 state memory isolation",
            Self::Dc5 => "Safe-idle on M4F heartbeat loss ≤12 ms",
            Self::Dc6 => "Intent attestation (HMAC-SHA256)",
        }
    }

    /// Evidence level
    pub fn evidence_level(&self) -> crate::EvidenceLevel {
        match self {
            Self::Dc1 | Self::Dc2 | Self::Dc3 | Self::Dc5 => crate::EvidenceLevel::L2,
            Self::Dc4 | Self::Dc6 => crate::EvidenceLevel::L1,
        }
    }
}

/// IPC latency measurement
#[derive(Debug, Clone, Copy)]
pub struct IpcLatency {
    /// Measured round-trip latency [µs]
    pub round_trip_us: f32,
    /// Lower bound from timing analysis [µs]
    pub lower_bound_us: f32,
    /// Excess attributable to bus contention [µs]
    pub contention_us: f32,
}

impl IpcLatency {
    /// Create from measurement
    ///
    /// Theorem 7.1: t_IPC,min = 14 / 168MHz = 83.3 ns = 0.083 µs [L1]
    /// Measured: t_IPC = 0.200 µs [L2]
    /// Excess: Δ_bus = 0.117 µs (+140%)
    pub fn measured() -> Self {
        Self {
            round_trip_us: 0.200, // [L2]
            lower_bound_us: 0.083, // [L1]
            contention_us: 0.117, // [L2]
        }
    }
}

/// Contract violation record
#[derive(Debug, Clone, Copy)]
pub struct ContractViolation {
    /// Which clause was violated
    pub clause: DcClause,
    /// Timestamp of violation [µs]
    pub timestamp: u64,
    /// Observed value (if applicable)
    pub observed: Option<f32>,
    /// Expected bound
    pub expected: Option<f32>,
}
