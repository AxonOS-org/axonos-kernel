//! Common Spatial Patterns (CSP) Filter
//!
//! WCET: 100.0 µs = 16,800 cycles / 168 MHz [L1]
//!
//! CSP is a spatial filtering technique that maximizes the variance
//! ratio between two motor imagery classes.
//!
//! For 4-class problems, we use one-vs-rest CSP filters.

use super::EegFrame;

/// CSP spatial filter
pub struct CspFilter {
    /// Number of channels
    channels: usize,
    /// CSP projection matrix (8 × 8)
    /// Pre-computed from training data
    projection: [[f32; 8]; 8],
    /// Log-variance features
    logvar: [f32; 8],
}

impl CspFilter {
    /// Create CSP filter with pre-computed projection
    pub fn new(channels: usize) -> Self {
        assert!(channels <= 8);

        // Identity initialization (will be replaced by trained projection)
        let mut projection = [[0.0f32; 8]; 8];
        for i in 0..channels {
            projection[i][i] = 1.0;
        }

        Self {
            channels,
            projection,
            logvar: [0.0; 8],
        }
    }

    /// Load pre-computed CSP projection matrix
    ///
    /// Matrix is computed offline from calibration data using
    /// generalized eigenvalue decomposition.
    pub fn load_projection(&mut self, proj: &[[f32; 8]; 8]) {
        self.projection = *proj;
    }

    /// Project EEG frame through CSP filters
    ///
    /// Returns log-variance features for LDA classification.
    pub fn project(&mut self, frame: EegFrame) -> [f32; 8] {
        let mut filtered = [0.0f32; 8];

        // Spatial filtering: y = W * x
        for i in 0..self.channels {
            for j in 0..self.channels {
                filtered[i] += self.projection[i][j] * (frame[j] as f32);
            }
        }

        // Compute log-variance features
        for i in 0..self.channels {
            let var = filtered[i] * filtered[i];
            self.logvar[i] = (var + 1e-6).ln();
        }

        self.logvar
    }

    /// Get current log-variance features
    pub fn features(&self) -> [f32; 8] {
        self.logvar
    }

    /// Reset
    pub fn reset(&mut self) {
        self.logvar = [0.0; 8];
    }
}
