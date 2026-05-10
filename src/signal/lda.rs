//! Linear Discriminant Analysis (LDA) Classifier
//!
//! WCET: 40.2 µs = 6,754 cycles / 168 MHz [L1]
//!
//! Simple LDA classifier for 4-class motor imagery.
//! Decision: argmax_c (w_c^T * x + b_c)

use super::MotorImageryClass;

/// LDA classifier
pub struct LdaClassifier {
    /// Number of classes
    num_classes: usize,
    /// Weight vectors (one per class)
    weights: [[f32; 8]; 4],
    /// Bias terms
    biases: [f32; 4],
    /// Decision threshold for "Idle" class
    confidence_threshold: f32,
}

/// Classification result with confidence
#[derive(Debug, Clone, Copy)]
pub struct Classification {
    /// Predicted class
    pub class: MotorImageryClass,
    /// Confidence score [0.0, 1.0]
    pub confidence: f32,
    /// Log-likelihood scores for all classes
    pub scores: [f32; 4],
}

impl LdaClassifier {
    /// Create LDA classifier
    pub fn new(num_classes: usize) -> Self {
        assert!(num_classes <= 4);

        Self {
            num_classes,
            weights: [[0.0; 8]; 4],
            biases: [0.0; 4],
            confidence_threshold: 0.5,
        }
    }

    /// Load trained LDA parameters
    pub fn load_weights(&mut self, weights: &[[f32; 8]; 4], biases: &[f32; 4]) {
        self.weights = *weights;
        self.biases = *biases;
    }

    /// Classify CSP features
    ///
    /// Returns MotorImageryClass::Idle if confidence below threshold.
    pub fn classify(&self, features: [f32; 8]) -> MotorImageryClass {
        let mut scores = [0.0f32; 4];

        // Compute discriminant functions: g_c(x) = w_c^T * x + b_c
        for c in 0..self.num_classes {
            let mut dot = self.biases[c];
            for i in 0..8 {
                dot += self.weights[c][i] * features[i];
            }
            scores[c] = dot;
        }

        // Find maximum score
        let mut max_score = scores[0];
        let mut max_class = 0;

        for c in 1..self.num_classes {
            if scores[c] > max_score {
                max_score = scores[c];
                max_class = c;
            }
        }

        // Compute confidence (softmax-like normalization)
        let exp_sum: f32 = scores.iter().map(|s| (s - max_score).exp()).sum();
        let confidence = 1.0 / exp_sum;

        if confidence < self.confidence_threshold {
            MotorImageryClass::Idle
        } else {
            match max_class {
                0 => MotorImageryClass::Left,
                1 => MotorImageryClass::Right,
                2 => MotorImageryClass::Feet,
                3 => MotorImageryClass::Tongue,
                _ => MotorImageryClass::Idle,
            }
        }
    }

    /// Classify with full result
    pub fn classify_full(&self, features: [f32; 8]) -> Classification {
        let mut scores = [0.0f32; 4];

        for c in 0..self.num_classes {
            let mut dot = self.biases[c];
            for i in 0..8 {
                dot += self.weights[c][i] * features[i];
            }
            scores[c] = dot;
        }

        let mut max_score = scores[0];
        let mut max_class = 0;

        for c in 1..self.num_classes {
            if scores[c] > max_score {
                max_score = scores[c];
                max_class = c;
            }
        }

        let exp_sum: f32 = scores.iter().map(|s| (s - max_score).exp()).sum();
        let confidence = 1.0 / exp_sum;

        let class = if confidence < self.confidence_threshold {
            MotorImageryClass::Idle
        } else {
            match max_class {
                0 => MotorImageryClass::Left,
                1 => MotorImageryClass::Right,
                2 => MotorImageryClass::Feet,
                3 => MotorImageryClass::Tongue,
                _ => MotorImageryClass::Idle,
            }
        };

        Classification { class, confidence, scores }
    }

    /// Reset
    pub fn reset(&mut self) {
        // Weights persist (trained parameters)
    }
}
