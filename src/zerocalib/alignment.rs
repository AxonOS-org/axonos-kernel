//! Euclidean Alignment for Domain Shift Reduction
//!
//! Stage 2: Align new session data to population model.
//!
//! P̃_k = P̄_new^(-1/2) P_k P̄_new^(-1/2)

use super::riemannian::{CovMatrix, identity, zeros, add, scale};

/// Euclidean alignment
pub struct EuclideanAlignment {
    /// Session mean covariance
    session_mean: CovMatrix,
    /// Alignment matrix: P̄_new^(-1/2)
    alignment_matrix: CovMatrix,
}

impl EuclideanAlignment {
    /// Create alignment from session data
    pub fn from_session(covariances: &[CovMatrix]) -> Self {
        // Compute session mean
        let session_mean = Self::compute_mean(covariances);

        // Compute P̄^(-1/2) (simplified: diagonal approximation)
        let alignment_matrix = Self::inverse_sqrt(&session_mean);

        Self {
            session_mean,
            alignment_matrix,
        }
    }

    /// Align covariance matrix
    pub fn align(&self, matrix: &CovMatrix) -> CovMatrix {
        // P̃ = A · P · A^T where A = P̄^(-1/2)
        let temp = super::riemannian::mul(&self.alignment_matrix, matrix);
        let aligned = super::riemannian::mul(&temp, &super::riemannian::transpose(&self.alignment_matrix));
        aligned
    }

    /// Compute mean covariance
    fn compute_mean(matrices: &[CovMatrix]) -> CovMatrix {
        let mut sum = zeros();
        for m in matrices {
            sum = add(&sum, m);
        }
        scale(&sum, 1.0 / matrices.len() as f32)
    }

    /// Compute inverse square root (diagonal approximation)
    fn inverse_sqrt(matrix: &CovMatrix) -> CovMatrix {
        let mut result = identity();
        for i in 0..8 {
            if matrix[i][i] > 0.0 {
                result[i][i] = 1.0 / micromath::F32(matrix[i][i]).sqrt().0;
            }
        }
        result
    }
}
