//! Raw full-step/two-half-step discrepancies in volume-average velocity and curl norms.
use super::kernel::Field;
use crate::{domain::Domain, spectral::modal, Complex64, SolverError};

/// Absolute and relative local budgets for velocity and vorticity respectively.
#[derive(Debug, Clone, Copy)]
pub struct Tolerances {
    /// Strictly positive absolute floors.
    pub absolute: [f64; 2],
    /// Nonnegative relative budgets.
    pub relative: [f64; 2],
}

impl Tolerances {
    /// Reject nonfinite, negative and zero-floor policies before an attempt.
    pub fn validate(self) -> Result<(), SolverError> {
        if self.absolute.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || self.relative.iter().any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(SolverError::InvalidStep);
        }
        Ok(())
    }
}

/// Empirical local discrepancy, with no Richardson divisor or certified-bound claim.
#[derive(Debug, Clone, Copy)]
pub struct Indicators {
    /// Raw velocity and vorticity volume-average L2 discrepancies.
    pub errors: [f64; 2],
    /// Discrepancy divided by each declared absolute-plus-relative budget.
    pub ratios: [f64; 2],
}

pub(crate) fn compare(
    domain: Domain,
    coarse: &Field,
    fine: &Field,
    tolerances: Tolerances,
) -> Result<Indicators, SolverError> {
    let layout = domain.layout();
    let mut errors = [0.0_f64; 2];
    let mut norms = [0.0_f64; 2];
    for index in 0..layout.half_len() {
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            continue;
        }
        let k = modal::wavevector(domain, layout.mode(position)?)?;
        let u = std::array::from_fn(|axis| fine[axis][index]);
        let difference = std::array::from_fn(|axis| fine[axis][index] - coarse[axis][index]);
        let weight = layout.weight(position)?.sqrt();
        accumulate(&mut norms[0], u, weight);
        accumulate(&mut errors[0], difference, weight);
        accumulate(&mut norms[1], modal::curl(k, u)?, weight);
        accumulate(&mut errors[1], modal::curl(k, difference)?, weight);
    }
    let mut ratios = [0.0; 2];
    for i in 0..2 {
        let budget = tolerances.absolute[i] + tolerances.relative[i] * norms[i];
        ratios[i] = errors[i] / budget;
        if !budget.is_finite() || !ratios[i].is_finite() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
    }
    Ok(Indicators { errors, ratios })
}

fn accumulate(norm: &mut f64, vector: [Complex64; 3], weight: f64) {
    for value in vector {
        *norm = norm.hypot(value.re * weight).hypot(value.im * weight);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrators::kernel::field;

    #[test]
    fn half_storage_weights_apply_to_both_complex_parts_and_both_error_channels() {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let mut coarse = field(48).unwrap();
        let mut fine = field(48).unwrap();
        coarse[0][1] = Complex64::new(1.0, 1.0);
        fine[0][1] = Complex64::new(3.0, 4.0);
        let result = compare(
            domain,
            &coarse,
            &fine,
            Tolerances {
                absolute: [1.0, 2.0],
                relative: [0.25, 0.125],
            },
        )
        .unwrap();
        let velocity = 26.0_f64.sqrt();
        let vorticity = std::f64::consts::TAU * velocity;
        assert!((result.errors[0] - velocity).abs() < 1e-14);
        assert!((result.errors[1] - vorticity).abs() < 1e-13);
        assert!((result.ratios[0] - velocity / (1.0 + 0.25 * 50.0_f64.sqrt())).abs() < 1e-14);
        assert!(
            (result.ratios[1]
                - vorticity / (2.0 + 0.125 * std::f64::consts::TAU * 50.0_f64.sqrt()))
            .abs()
                < 1e-14
        );
    }

    #[test]
    fn overflowing_discrepancy_is_refused_even_with_a_finite_fine_state_budget() {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let mut coarse = field(48).unwrap();
        let fine = field(48).unwrap();
        for component in &mut coarse {
            component[0] = Complex64::new(f64::MAX, 0.0);
        }
        let result = compare(
            domain,
            &coarse,
            &fine,
            Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
        );
        assert_eq!(
            result.unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
        let mut large_fine = field(48).unwrap();
        large_fine[0][0] = Complex64::new(100.0, 0.0);
        let result = compare(
            domain,
            &fine,
            &large_fine,
            Tolerances {
                absolute: [1.0; 2],
                relative: [f64::MAX; 2],
            },
        );
        assert_eq!(
            result.unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
    }
}
