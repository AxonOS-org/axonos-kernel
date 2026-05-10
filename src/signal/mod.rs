//! Signal Processing Pipeline
//!
//! Five-stage pipeline from ADC DMA to intent classification:
//! 1. Kalman state estimator (8-ch)
//! 2. FIR bandpass filter (order 64, 8-ch)
//! 3. Notch filter (50 Hz + 60 Hz)
//! 4. Artifact rejection (±120 µV)
//! 5. CSP spatial filter (8 × 8)
//! 6. LDA classifier
//!
//! Pipeline subtotal WCET: 640.2 µs [L1] (nominal)
//! L2-inferred binding WCET: 818 µs [L2]

pub mod pipeline;
pub mod fir;
pub mod kalman;
pub mod csp;
pub mod lda;
pub mod notch;
pub mod artifact;

pub use pipeline::SignalPipeline;
pub use fir::FirFilter;
pub use kalman::KalmanEstimator;
pub use csp::CspFilter;
pub use lda::LdaClassifier;

/// EEG sample frame: 8 channels × 24 bits
pub type EegFrame = [i32; 8];

/// Classifier output (4-class motor imagery)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorImageryClass {
    /// Left hand
    Left,
    /// Right hand
    Right,
    /// Both feet
    Feet,
    /// Tongue
    Tongue,
    /// No classification / idle
    Idle,
}

/// Epoch counter and timing
#[derive(Debug, Clone, Copy)]
pub struct Epoch {
    /// Epoch index (monotonically increasing)
    pub index: u64,
    /// DWT cycle count at epoch start
    pub start_cycles: u64,
    /// DWT cycle count at pipeline completion
    pub end_cycles: u64,
    /// Pipeline execution time [µs]
    pub elapsed_us: u32,
}

impl Epoch {
    /// Create new epoch
    pub fn new(index: u64, start_cycles: u64) -> Self {
        Self {
            index,
            start_cycles,
            end_cycles: 0,
            elapsed_us: 0,
        }
    }

    /// Mark epoch complete
    pub fn complete(&mut self, end_cycles: u64) {
        self.end_cycles = end_cycles;
        // DWT resolution: ~5.95 ns per cycle at 168 MHz
        self.elapsed_us = ((end_cycles - self.start_cycles) * 595) / 100_000;
    }
}
