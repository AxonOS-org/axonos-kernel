//! Notch Filter (50 Hz + 60 Hz)
//!
//! WCET: 60.0 µs = 10,080 cycles / 168 MHz [L1]
//!
//! Second-order IIR notch filters for powerline interference removal.
//! Separate notches for 50 Hz (Europe/Asia) and 60 Hz (Americas).

use super::EegFrame;

/// Notch filter for powerline frequencies
pub struct NotchFilter {
    /// Sampling rate [Hz]
    fs: f32,
    /// Notch frequencies [Hz]
    freqs: [f32; 2],
    /// Filter states (biquad sections)
    states: [[f32; 4]; 8], // 4 state variables per channel (2 biquads)
    /// Filter coefficients (pre-computed)
    coeffs: [[f32; 6]; 2], // 6 coeffs per biquad (b0, b1, b2, a1, a2, gain)
}

impl NotchFilter {
    /// Create notch filter
    pub fn new(sampling_rate: u32) -> Self {
        let fs = sampling_rate as f32;
        let f50 = 50.0;
        let f60 = 60.0;

        // Compute biquad coefficients for each notch
        let coeffs50 = Self::notch_coefficients(fs, f50, 30.0); // Q=30
        let coeffs60 = Self::notch_coefficients(fs, f60, 30.0);

        Self {
            fs,
            freqs: [f50, f60],
            states: [[0.0; 4]; 8],
            coeffs: [coeffs50, coeffs60],
        }
    }

    /// Compute notch filter coefficients (biquad)
    ///
    /// H(s) = (s^2 + ω0^2) / (s^2 + (ω0/Q)s + ω0^2)
    fn notch_coefficients(fs: f32, f0: f32, q: f32) -> [f32; 6] {
        let w0 = 2.0 * core::f32::consts::PI * f0 / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        // Normalize
        [b0/a0, b1/a0, b2/a0, a1/a0, a2/a0, 1.0]
    }

    /// Process frame through notch filters
    pub fn process(&mut self, frame: EegFrame) -> EegFrame {
        let mut output = frame;

        for ch in 0..8 {
            // Apply 50 Hz notch
            output[ch] = self.apply_biquad(ch, 0, output[ch]);
            // Apply 60 Hz notch
            output[ch] = self.apply_biquad(ch, 1, output[ch]);
        }

        output
    }

    /// Apply single biquad section
    fn apply_biquad(&mut self, ch: usize, section: usize, input: i32) -> i32 {
        let x = input as f32;
        let c = &self.coeffs[section];
        let s = &mut self.states[ch];

        // Direct Form II transposed
        let y = c[0] * x + s[section * 2];
        s[section * 2] = c[1] * x + c[3] * y + s[section * 2 + 1];
        s[section * 2 + 1] = c[2] * x + c[4] * y;

        y as i32
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.states = [[0.0; 4]; 8];
    }
}
