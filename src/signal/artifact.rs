//! Artifact Rejection
//!
//! WCET: 40.0 µs = 6,720 cycles / 168 MHz [L1]
//!
//! Simple threshold-based artifact rejection: if any channel exceeds
//! ±120 µV, the epoch is flagged as artifact-contaminated.
//!
//! This is a conservative approach; more sophisticated methods
//! (ICA, wavelet-based) are future work.

use super::EegFrame;

/// Artifact rejection configuration
pub struct ArtifactRejection {
    /// Threshold [µV]
    threshold_uv: i32,
    /// Consecutive artifact counter (for hysteresis)
    artifact_count: u32,
    /// Maximum consecutive artifacts before session quality degraded
    max_consecutive: u32,
}

/// Artifact detection result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactResult {
    /// Clean epoch
    Clean,
    /// Artifact detected (single channel)
    SingleChannel { channel: usize, amplitude_uv: i32 },
    /// Multiple channels contaminated
    MultiChannel,
    /// Session quality degraded (too many consecutive artifacts)
    SessionDegraded,
}

impl ArtifactRejection {
    /// Create artifact rejector
    pub fn new(threshold_uv: i32) -> Self {
        Self {
            threshold_uv,
            artifact_count: 0,
            max_consecutive: 10,
        }
    }

    /// Check if frame should be rejected
    ///
    /// Returns true if artifact detected (epoch should be skipped)
    pub fn reject(&mut self, frame: EegFrame) -> bool {
        let mut artifact_channels = 0;

        for ch in 0..8 {
            let abs_val = frame[ch].abs();
            if abs_val > self.threshold_uv {
                artifact_channels += 1;
            }
        }

        let is_artifact = artifact_channels > 0;

        if is_artifact {
            self.artifact_count += 1;
        } else {
            self.artifact_count = 0;
        }

        is_artifact
    }

    /// Detailed artifact analysis (for diagnostics)
    pub fn analyze(&self, frame: EegFrame) -> ArtifactResult {
        let mut artifact_channels = 0;
        let mut first_channel = 0;
        let mut max_amp = 0;

        for ch in 0..8 {
            let abs_val = frame[ch].abs();
            if abs_val > self.threshold_uv {
                if artifact_channels == 0 {
                    first_channel = ch;
                    max_amp = abs_val;
                }
                artifact_channels += 1;
            }
        }

        if artifact_channels == 0 {
            ArtifactResult::Clean
        } else if artifact_channels == 1 {
            ArtifactResult::SingleChannel { 
                channel: first_channel, 
                amplitude_uv: max_amp 
            }
        } else if self.artifact_count >= self.max_consecutive {
            ArtifactResult::SessionDegraded
        } else {
            ArtifactResult::MultiChannel
        }
    }

    /// Check if session quality is degraded
    pub fn is_session_degraded(&self) -> bool {
        self.artifact_count >= self.max_consecutive
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.artifact_count = 0;
    }
}
