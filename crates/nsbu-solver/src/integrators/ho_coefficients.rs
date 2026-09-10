//! Independent recurrence/Taylor construction of the reviewed five-stage HO tableau.
use crate::SolverError;

/// Dimensionless explicit tableau; every stage/source weight is multiplied by dt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoCoefficients {
    /// Full-step exponential.
    pub exponential: f64,
    /// Half-step exponential used by stages two, three and five.
    pub half_exponential: f64,
    /// Strictly lower triangular stage weights, with unused entries exactly zero.
    pub rows: [[f64; 5]; 5],
    /// Final source weights; stages two and three have exact zero weights.
    pub weights: [f64; 5],
    /// Absolute analytic Taylor truncation allowance, excluding floating roundoff.
    pub truncation_bound: f64,
}

impl HoCoefficients {
    /// Evaluate finite dissipative arguments using independent phi recurrences.
    pub fn new(z: f64) -> Result<Self, SolverError> {
        if !z.is_finite() || z > 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let [p1, p2, p3] = phi(z).0;
        let ([h1, h2, h3], truncation_bound) = phi(z / 2.0);
        let a52 = h2 / 2.0 - p3 + p2 / 4.0 - h3 / 2.0;
        let a54 = h2 / 4.0 - a52;
        Ok(Self {
            exponential: z.exp(),
            half_exponential: (z / 2.0).exp(),
            rows: [
                [0.0; 5],
                [h1 / 2.0, 0.0, 0.0, 0.0, 0.0],
                [h1 / 2.0 - h2, h2, 0.0, 0.0, 0.0],
                [p1 - 2.0 * p2, p2, p2, 0.0, 0.0],
                [h1 / 2.0 - 2.0 * a52 - a54, a52, a52, a54, 0.0],
            ],
            weights: [
                p1 - 3.0 * p2 + 4.0 * p3,
                0.0,
                0.0,
                -p2 + 4.0 * p3,
                4.0 * p2 - 8.0 * p3,
            ],
            // If the full argument uses Taylor, the half argument also does.
            // The half-argument report therefore bounds all tableau truncation.
            truncation_bound,
        })
    }
}

fn phi(z: f64) -> ([f64; 3], f64) {
    if z < -1.0 {
        let first = z.exp_m1() / z;
        let second = (first - 1.0) / z;
        ([first, second, (second - 0.5) / z], 0.0)
    } else {
        (
            [
                series(z, 1, 1.0),
                series(z, 2, 0.5),
                series(z, 3, 1.0 / 6.0),
            ],
            1e-23,
        )
    }
}

fn series(z: f64, order: usize, initial: f64) -> f64 {
    let mut term = initial;
    let mut total = term;
    // m=0..23: each phi tail on [-1,0] is below e/25!.
    for index in 1..24 {
        term *= z / (index + order) as f64;
        total += term;
    }
    total
}
