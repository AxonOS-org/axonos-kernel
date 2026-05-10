//! Signal Pipeline Orchestrator
//!
//! Zero-copy signal path from ADC DMA to classifier.
//! No heap allocation on hot path.
//!
//! ## Pipeline Stages (Table 3)
//!
//! | Stage | C_i (µs) | Derivation |
//! |-------|----------|------------|
//! | Kalman state estimator (8-ch) | 80.0 | 13,440 cycles / 168 MHz |
//! | FIR bandpass (order 64, 8-ch) | 320.0 | ≈40 µs/ch (Remark 5.5) |
//! | Notch filter (50 Hz + 60 Hz) | 60.0 | 10,080 cycles / 168 MHz |
//! | Artifact rejection (±120 µV) | 40.0 | 6,720 cycles / 168 MHz |
//! | CSP spatial filter (8 × 8) | 100.0 | 16,800 cycles / 168 MHz |
//! | LDA classifier | 40.2 | 6,754 cycles / 168 MHz |
//! | **Pipeline subtotal** | **640.2** | incl. SPSC push overhead |

use super::{EegFrame, MotorImageryClass, Epoch};
use crate::ringbuf::SpscRingBuffer;
use crate::config;

/// Signal pipeline state
pub struct SignalPipeline {
    /// Kalman estimator
    kalman: crate::signal::kalman::KalmanEstimator,
    /// FIR bandpass filter bank
    fir: crate::signal::fir::FirFilter,
    /// Notch filter
    notch: crate::signal::notch::NotchFilter,
    /// Artifact rejection
    artifact: crate::signal::artifact::ArtifactRejection,
    /// CSP spatial filter
    csp: crate::signal::csp::CspFilter,
    /// LDA classifier
    lda: crate::signal::lda::LdaClassifier,
    /// Output ring buffer (to IPC)
    output: SpscRingBuffer<MotorImageryClass>,
    /// Current epoch
    current_epoch: Option<Epoch>,
    /// Pipeline execution counter
    epoch_count: u64,
    /// Maximum observed execution time [µs]
    wcet_observed: u32,
}

/// Pipeline configuration
#[derive(Debug, Clone, Copy)]
pub struct PipelineConfig {
    /// FIR filter order
    pub fir_order: usize,
    /// Number of EEG channels
    pub channels: usize,
    /// ADC sampling rate [SPS]
    pub sampling_rate: u32,
    /// Artifact rejection threshold [µV]
    pub artifact_threshold_uv: i32,
    /// Number of LDA classes
    pub num_classes: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            fir_order: config::FIR_ORDER,
            channels: config::EEG_CHANNELS,
            sampling_rate: config::ADC_SPS,
            artifact_threshold_uv: 120,
            num_classes: 4,
        }
    }
}

impl SignalPipeline {
    /// Create new signal pipeline
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            kalman: crate::signal::kalman::KalmanEstimator::new(config.channels),
            fir: crate::signal::fir::FirFilter::new(config.fir_order, config.channels),
            notch: crate::signal::notch::NotchFilter::new(config.sampling_rate),
            artifact: crate::signal::artifact::ArtifactRejection::new(config.artifact_threshold_uv),
            csp: crate::signal::csp::CspFilter::new(config.channels),
            lda: crate::signal::lda::LdaClassifier::new(config.num_classes),
            output: SpscRingBuffer::new(),
            current_epoch: None,
            epoch_count: 0,
            wcet_observed: 0,
        }
    }

    /// Process one epoch of EEG data
    ///
    /// Called by scheduler at each ADC DMA completion interrupt.
    /// Must complete within 4 ms deadline.
    ///
    /// # Arguments
    /// * `frame` — Raw EEG frame from ADC DMA (8 channels × 24-bit)
    /// * `epoch` — Epoch metadata for timing measurement
    ///
    /// # Returns
    /// Classifier output or None if artifact rejected
    pub fn process(&mut self, frame: EegFrame, epoch: Epoch) -> Option<MotorImageryClass> {
        self.current_epoch = Some(epoch);
        self.epoch_count += 1;

        // Stage 1: Kalman state estimation
        let estimated = self.kalman.update(frame);

        // Stage 2: FIR bandpass filtering (dominant stage: ~50% of WCET)
        let filtered = self.fir.process(estimated);

        // Stage 3: Notch filter (50 Hz + 60 Hz powerline)
        let notched = self.notch.process(filtered);

        // Stage 4: Artifact rejection (±120 µV threshold)
        if self.artifact.reject(notched) {
            // Artifact detected — skip classification
            return None;
        }

        // Stage 5: CSP spatial filtering
        let spatial = self.csp.project(notched);

        // Stage 6: LDA classification
        let class = self.lda.classify(spatial);

        // Push to output ring buffer (zero-copy via SPSC)
        let _ = self.output.try_push(class);

        // Update WCET observation
        if let Some(ref mut e) = self.current_epoch {
            let elapsed = e.elapsed_us;
            if elapsed > self.wcet_observed {
                self.wcet_observed = elapsed;
            }
        }

        Some(class)
    }

    /// Get observed WCET [µs]
    pub fn observed_wcet(&self) -> u32 {
        self.wcet_observed
    }

    /// Get epoch count
    pub fn epoch_count(&self) -> u64 {
        self.epoch_count
    }

    /// Reset pipeline state (e.g., after session change)
    pub fn reset(&mut self) {
        self.kalman.reset();
        self.fir.reset();
        self.notch.reset();
        self.artifact.reset();
        self.csp.reset();
        self.lda.reset();
        self.epoch_count = 0;
        self.wcet_observed = 0;
    }
}

/// Pipeline stage trait for modular composition
trait PipelineStage {
    type Input;
    type Output;
    fn process(&mut self, input: Self::Input) -> Self::Output;
    fn reset(&mut self);
}
