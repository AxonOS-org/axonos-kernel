//! Riemannian Geometry for EEG Covariance Matrices
//!
//! EEG covariance matrices lie on the symmetric positive definite (SPD) manifold.
//!
//! AIRM metric: d_AIRM(P, Q) = ||log(P^(-1/2) Q P^(-1/2))||_F

/// 8x8 covariance matrix (fixed size for no_std)
pub type CovMatrix = [[f32; 8]; 8];

/// Identity matrix
pub fn identity() -> CovMatrix {
    let mut m = [[0.0f32; 8]; 8];
    for i in 0..8 {
        m[i][i] = 1.0;
    }
    m
}

/// Zero matrix
pub fn zeros() -> CovMatrix {
    [[0.0f32; 8]; 8]
}

/// Matrix addition
pub fn add(a: &CovMatrix, b: &CovMatrix) -> CovMatrix {
    let mut result = zeros();
    for i in 0..8 {
        for j in 0..8 {
            result[i][j] = a[i][j] + b[i][j];
        }
    }
    result
}

/// Matrix subtraction
pub fn sub(a: &CovMatrix, b: &CovMatrix) -> CovMatrix {
    let mut result = zeros();
    for i in 0..8 {
        for j in 0..8 {
            result[i][j] = a[i][j] - b[i][j];
        }
    }
    result
}

/// Scalar multiplication
pub fn scale(m: &CovMatrix, s: f32) -> CovMatrix {
    let mut result = zeros();
    for i in 0..8 {
        for j in 0..8 {
            result[i][j] = m[i][j] * s;
        }
    }
    result
}

/// Matrix multiplication (8x8)
pub fn mul(a: &CovMatrix, b: &CovMatrix) -> CovMatrix {
    let mut result = zeros();
    for i in 0..8 {
        for j in 0..8 {
            let mut sum = 0.0f32;
            for k in 0..8 {
                sum += a[i][k] * b[k][j];
            }
            result[i][j] = sum;
        }
    }
    result
}

/// Matrix transpose
pub fn transpose(m: &CovMatrix) -> CovMatrix {
    let mut result = zeros();
    for i in 0..8 {
        for j in 0..8 {
            result[j][i] = m[i][j];
        }
    }
    result
}

/// Frobenius norm
pub fn frobenius_norm(m: &CovMatrix) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..8 {
        for j in 0..8 {
            sum += m[i][j] * m[i][j];
        }
    }
    micromath::F32(sum).sqrt().0
}

/// Riemannian distance (simplified approximation)
/// d_AIRM(P, Q) ≈ ||log(P^(-1) Q)||_F
pub fn riemannian_distance(p: &CovMatrix, q: &CovMatrix) -> f32 {
    // Simplified: use Euclidean distance as approximation
    // Full AIRM requires matrix log and eigenvalue decomposition
    let diff = sub(p, q);
    frobenius_norm(&diff)
}

/// Frechet mean (geodesic gradient descent)
pub fn frechet_mean(matrices: &[CovMatrix], gamma: f32) -> CovMatrix {
    if matrices.is_empty() {
        return identity();
    }

    // Initialize with arithmetic mean
    let mut mean = zeros();
    for m in matrices {
        mean = add(&mean, m);
    }
    mean = scale(&mean, 1.0 / matrices.len() as f32);

    // Geodesic gradient descent
    for _ in 0..10 {
        let mut gradient = zeros();
        for m in matrices {
            let diff = sub(m, &mean);
            gradient = add(&gradient, &diff);
        }
        gradient = scale(&gradient, gamma / matrices.len() as f32);
        mean = add(&mean, &gradient);
    }

    mean
}

/// Matrix logarithm (diagonal approximation)
pub fn matrix_log(m: &CovMatrix) -> CovMatrix {
    // Approximation: log(I + (M - I)) ≈ M - I for M near identity
    let mut result = *m;
    for i in 0..8 {
        result[i][i] = micromath::F32(result[i][i]).ln().0;
    }
    result
}

/// Matrix exponential (diagonal approximation)
pub fn matrix_exp(m: &CovMatrix) -> CovMatrix {
    let mut result = *m;
    for i in 0..8 {
        result[i][i] = micromath::F32(result[i][i]).exp().0;
    }
    result
}

/// Compute covariance matrix from EEG samples
pub fn compute_covariance(samples: &[[f32; 8]]) -> CovMatrix {
    let n = samples.len() as f32;
    let mut mean = [0.0f32; 8];

    // Compute mean
    for sample in samples {
        for i in 0..8 {
            mean[i] += sample[i];
        }
    }
    for i in 0..8 {
        mean[i] /= n;
    }

    // Compute covariance
    let mut cov = zeros();
    for sample in samples {
        for i in 0..8 {
            for j in 0..8 {
                cov[i][j] += (sample[i] - mean[i]) * (sample[j] - mean[j]);
            }
        }
    }

    scale(&cov, 1.0 / (n - 1.0))
}
