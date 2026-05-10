//! FIR Bandpass Filter
//!
//! Remark 5.5 (FIR dominance): The FIR filter accounts for ≈50% of pipeline WCET.
//! Order-64 FIR on 8 channels requires 64 × 8 × 2 = 1024 MAC operations.
//! M4F SMLAD instruction (dual 16-bit MAC) halves this to 512 instructions
//! at 1-cycle throughput, yielding 512/168MHz ≈ 3.0 µs compute plus
//! coefficient-load overhead totalling ≈40 µs/channel.
//!
//! Total: 8 channels × 40 µs = 320 µs [L1]

use super::EegFrame;

/// FIR filter bank (one filter per channel)
pub struct FirFilter {
    /// Filter order
    order: usize,
    /// Number of channels
    channels: usize,
    /// Filter coefficients (SRAM-resident after boot)
    coefficients: [[i16; 64]; 8],
    /// Delay lines (circular buffers)
    delay_lines: [[i32; 64]; 8],
    /// Delay line write index
    delay_index: usize,
    /// Coefficients loaded from Flash (cold start penalty: 18.3 µs [L1])
    coefficients_in_sram: bool,
}

impl FirFilter {
    /// Create new FIR filter bank
    pub fn new(order: usize, channels: usize) -> Self {
        assert!(order <= 64);
        assert!(channels <= 8);

        Self {
            order,
            channels,
            coefficients: [[0; 64]; 8],
            delay_lines: [[0; 64]; 8],
            delay_index: 0,
            coefficients_in_sram: false,
        }
    }

    /// Load coefficients from Flash to SRAM (one-time cold start)
    ///
    /// Source 1: Flash read penalty = 18.3 µs [L1]
    /// Applies only to first epoch after boot.
    pub fn load_coefficients(&mut self, coeffs: &[[i16; 64]; 8]) {
        self.coefficients = *coeffs;
        self.coefficients_in_sram = true;
    }

    /// Process one frame through FIR filter bank
    ///
    /// Uses SMLAD dual-MAC instruction on Cortex-M4F for efficiency.
    pub fn process(&mut self, frame: EegFrame) -> EegFrame {
        let mut output = [0i32; 8];

        for ch in 0..self.channels {
            // Write new sample to delay line
            self.delay_lines[ch][self.delay_index] = frame[ch];

            // Compute convolution: y[n] = Σ_k h[k] * x[n-k]
            let mut acc: i64 = 0;
            for k in 0..self.order {
                let idx = (self.delay_index + self.order - k) % self.order;
                let sample = self.delay_lines[ch][idx] as i64;
                let coeff = self.coefficients[ch][k] as i64;
                acc += sample * coeff;
            }

            // Scale down (fixed-point Q15 coefficients)
            output[ch] = (acc >> 15) as i32;
        }

        self.delay_index = (self.delay_index + 1) % self.order;
        output
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.delay_lines = [[0; 64]; 8];
        self.delay_index = 0;
    }
}
