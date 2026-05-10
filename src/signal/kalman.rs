//! Kalman State Estimator
//!
//! WCET: 80.0 µs = 13,440 cycles / 168 MHz [L1]
//!
//! Simple Kalman filter per channel for DC drift removal and
//! baseline estimation before bandpass filtering.

use super::EegFrame;

/// Kalman estimator per channel
pub struct KalmanEstimator {
    /// Number of channels
    channels: usize,
    /// State estimate per channel [µV]
    state: [f32; 8],
    /// Error covariance per channel
    covariance: [f32; 8],
    /// Process noise variance
    q: f32,
    /// Measurement noise variance
    r: f32,
}

impl KalmanEstimator {
    /// Create new Kalman estimator
    pub fn new(channels: usize) -> Self {
        assert!(channels <= 8);
        Self {
            channels,
            state: [0.0; 8],
            covariance: [1.0; 8],
            q: 0.01,  // Process noise
            r: 100.0, // Measurement noise (ADC quantization)
        }
    }

    /// Update estimator with new measurement
    ///
    /// Prediction: x̂_k|k-1 = x̂_k-1|k-1
    ///             P_k|k-1 = P_k-1|k-1 + Q
    /// Update:     K_k = P_k|k-1 / (P_k|k-1 + R)
    ///             x̂_k|k = x̂_k|k-1 + K_k * (z_k - x̂_k|k-1)
    ///             P_k|k = (1 - K_k) * P_k|k-1
    pub fn update(&mut self, measurement: EegFrame) -> EegFrame {
        let mut output = [0i32; 8];

        for ch in 0..self.channels {
            // Prediction
            let p_pred = self.covariance[ch] + self.q;

            // Kalman gain
            let k = p_pred / (p_pred + self.r);

            // Update
            let z = measurement[ch] as f32;
            let innovation = z - self.state[ch];
            self.state[ch] += k * innovation;
            self.covariance[ch] = (1.0 - k) * p_pred;

            // Output: high-pass component (measurement - estimate)
            output[ch] = innovation as i32;
        }

        output
    }

    /// Reset estimator
    pub fn reset(&mut self) {
        self.state = [0.0; 8];
        self.covariance = [1.0; 8];
    }
}
