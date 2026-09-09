//! Bounded binary64 Cox–Matthews coefficients for finite dissipative modal arguments.
use crate::SolverError;

/// Dimensionless coefficients: multiply q and weights by the physical step duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmCoefficients {
    /// Full-step exponential.
    pub exponential: f64,
    /// Half-step exponential.
    pub half_exponential: f64,
    /// Half-step phi coefficient, phi_1(z/2)/2.
    pub q: f64,
    /// Weights for n1, na+nb, nc respectively.
    pub weights: [f64; 3],
    /// Absolute analytic truncation bound; does not enclose binary64 roundoff.
    pub truncation_bound: f64,
}

impl CmCoefficients {
    /// Evaluate the reviewed three-branch policy without forming an overflowing z cubed.
    pub fn new(z: f64) -> Result<Self, SolverError> {
        if !z.is_finite() || z > 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let (weights, mut truncation_bound) = if z >= -1.0 {
            let [p1, p2, p3] = [series(z, 1), series(z, 2), series(z, 3)];
            (
                [
                    p1 - 3.0 * p2 + 4.0 * p3,
                    2.0 * p2 - 4.0 * p3,
                    -p2 + 4.0 * p3,
                ],
                1e-17,
            )
        } else if z <= -50.0 {
            let r = 1.0 / z;
            (
                [
                    -r * r * (1.0 + 4.0 * r),
                    2.0 * r * r * (1.0 + 2.0 * r),
                    -r * (1.0 + 3.0 * r + 4.0 * r * r),
                ],
                1e-22,
            )
        } else {
            let e = z.exp();
            let denominator = z * z * z;
            (
                [
                    (e * (z * z - 3.0 * z + 4.0) - z - 4.0) / denominator,
                    2.0 * (e * (z - 2.0) + z + 2.0) / denominator,
                    (e * (4.0 - z) - z * z - 3.0 * z - 4.0) / denominator,
                ],
                0.0,
            )
        };
        let half = z / 2.0;
        let q = if half >= -1.0 {
            truncation_bound = 1e-17;
            series(half, 1) / 2.0
        } else {
            half.exp_m1() / half / 2.0
        };
        Ok(Self {
            exponential: z.exp(),
            half_exponential: half.exp(),
            q,
            weights,
            truncation_bound,
        })
    }
}

fn series(z: f64, order: usize) -> f64 {
    let mut term = match order {
        1 => 1.0,
        2 => 0.5,
        _ => 1.0 / 6.0,
    };
    let mut sum = term;
    // Nineteen terms, m=0..18; absolute remainder at |z|<=1 is < e/20!.
    for m in 1..19 {
        term *= z / (m + order) as f64;
        sum += term;
    }
    sum
}
