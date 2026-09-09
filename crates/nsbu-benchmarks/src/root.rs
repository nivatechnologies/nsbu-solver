//! Safeguarded binary64 solve of q - z² q^(1/4) = tau, independent of jet derivatives.
use crate::BenchmarkError;

/// Floating scalar solve evidence; endpoints and error estimates are not certified enclosures.
#[derive(Debug, Clone, Copy)]
pub struct RootReport {
    /// Positive approximate root.
    pub value: f64,
    /// Last floating bracket endpoints.
    pub bracket: [f64; 2],
    /// Recomputed signed residual estimate.
    pub residual: f64,
    /// Absolute root-error estimate including a conservative local roundoff allowance.
    pub error_estimate: f64,
    /// Number of safeguarded iterations consumed; the exact axis consumes zero.
    pub iterations: usize,
}

/// Solve in the bounded v2 clock range, with at most 128 iterations.
pub fn solve(z: f64, tau: f64, max_iterations: usize) -> Result<RootReport, BenchmarkError> {
    let minimum = f64::from_bits(889_u64 << 52); // 2^-134, smallest remaining tick at target=2^-7.
    if !z.is_finite()
        || z.abs() > 0.5
        || !tau.is_finite()
        || !(minimum..=1.0 / 128.0).contains(&tau)
        || !(1..=128).contains(&max_iterations)
    {
        return Err(BenchmarkError::InvalidInput);
    }
    if z == 0.0 {
        return Ok(RootReport {
            value: tau,
            bracket: [tau, tau],
            residual: 0.0,
            error_estimate: 0.0,
            iterations: 0,
        });
    }
    let mut lower = tau.max(z.abs().powf(8.0 / 3.0));
    let mut upper = 4.0 * lower;
    let mut q = (lower + upper) / 2.0;
    for iterations in 1..=max_iterations {
        let fourth = q.sqrt().sqrt();
        let residual = (-z * z).mul_add(fourth, q) - tau;
        let allowance = 8.0 * f64::EPSILON * q;
        if residual.abs() <= allowance {
            return Ok(RootReport {
                value: q,
                bracket: [lower, upper],
                residual,
                error_estimate: (residual.abs() + allowance) * 4.0 / 3.0,
                iterations,
            });
        }
        (q, lower, upper) = refine(q, lower, upper, z, fourth, residual);
    }
    Err(BenchmarkError::RootWorkExhausted)
}

// Separating one safeguarded update permits exercising both bracket signs and
// endpoint fallbacks independently of the initial-guess convergence history.
fn refine(
    q: f64,
    mut lower: f64,
    mut upper: f64,
    z: f64,
    fourth: f64,
    residual: f64,
) -> (f64, f64, f64) {
    if residual < 0.0 {
        lower = q;
    } else {
        upper = q;
    }
    let slope = 1.0 - z * z * fourth / (4.0 * q);
    let proposed = q - residual / slope;
    let next = if lower < proposed && proposed < upper {
        proposed
    } else {
        (lower + upper) / 2.0
    };
    (next, lower, upper)
}

#[cfg(test)]
mod tests {
    use super::refine;

    #[test]
    fn signed_bracket_updates_and_newton_safeguards() {
        assert_eq!(refine(16.0, 8.0, 32.0, 2.0, 2.0, 1.75), (14.0, 8.0, 16.0));
        assert_eq!(refine(16.0, 8.0, 32.0, 2.0, 2.0, -1.75), (18.0, 16.0, 32.0));
        assert_eq!(refine(16.0, 8.0, 32.0, 2.0, 2.0, 0.0), (12.0, 8.0, 16.0));
        // q=1,z=1 gives slope=3/4, so the Newton correction is independently exact.
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, 0.375), (0.5, 0.0, 1.0));
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, -0.375), (1.5, 1.0, 2.0));
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, 1.5), (0.5, 0.0, 1.0));
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, -1.5), (1.5, 1.0, 2.0));
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, 0.75), (0.5, 0.0, 1.0));
        assert_eq!(refine(1.0, 0.0, 2.0, 1.0, 1.0, -0.75), (1.5, 1.0, 2.0));
    }
}
