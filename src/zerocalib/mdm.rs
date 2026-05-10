//! Minimum Distance to Mean (MDM) Classifier
//!
//! Stage 1: Universal MDM classifier pre-trained on N_s = 127 subjects.

use super::riemannian::{CovMatrix, riemannian_distance};

/// MDM classifier
pub struct MdmClassifier {
    /// Class centroids on SPD manifold
    centroids: [CovMatrix; 4],
    /// Number of classes
    num_classes: usize,
}

/// Classification result
#[derive(Debug, Clone, Copy)]
pub struct MdmResult {
    pub predicted_class: usize,
    pub distances: [f32; 4],
    pub confidence: f32,
}

impl MdmClassifier {
    /// Create MDM classifier with pre-trained centroids
    pub fn new(centroids: [CovMatrix; 4]) -> Self {
        Self {
            centroids,
            num_classes: 4,
        }
    }

    /// Classify covariance matrix
    pub fn classify(&self, matrix: &CovMatrix) -> MdmResult {
        // Compute Riemannian distances to all centroids
        let mut distances = [0.0f32; 4];
        for i in 0..self.num_classes {
            distances[i] = riemannian_distance(matrix, &self.centroids[i]);
        }

        // Find minimum distance
        let mut min_idx = 0;
        let mut min_dist = distances[0];
        for i in 1..self.num_classes {
            if distances[i] < min_dist {
                min_dist = distances[i];
                min_idx = i;
            }
        }

        // Confidence: softmax of inverse distances
        let exp_sum: f32 = distances.iter().map(|d| (-d).exp()).sum();
        let confidence = (-min_dist).exp() / exp_sum;

        MdmResult {
            predicted_class: min_idx,
            distances,
            confidence,
        }
    }
}
