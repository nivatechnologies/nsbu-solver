//! Validate both self-conjugate planes of a general R2C transform input.
use crate::{domain::Layout, Complex64, SolverError};

pub(super) fn validate(layout: Layout, values: &[Complex64]) -> Result<(), SolverError> {
    finite(values)?;
    let [nx, ny, nz] = layout.dimensions();
    for i in 0..nx {
        for j in 0..ny {
            for k in [0, nz / 2] {
                let a = values[layout.index([i, j, k])?];
                let b = values[layout.index([(nx - i) % nx, (ny - j) % ny, k])?].conj();
                check_pair(a, b)?;
            }
        }
    }
    Ok(())
}

fn check_pair(a: Complex64, b: Complex64) -> Result<(), SolverError> {
    let scale =
        a.re.abs()
            .max(a.im.abs())
            .max(b.re.abs())
            .max(b.im.abs())
            .max(1.0);
    let difference = a - b;
    let tolerance = 64.0 * f64::EPSILON * scale;
    if difference.re.abs() > tolerance || difference.im.abs() > tolerance {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}

pub(super) fn finite(values: &[Complex64]) -> Result<(), SolverError> {
    if values
        .iter()
        .any(|v| !v.re.is_finite() || !v.im.is_finite())
    {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_components_and_relative_pair_boundaries_are_checked_separately() {
        for value in [
            Complex64::new(f64::INFINITY, 0.0),
            Complex64::new(0.0, f64::NAN),
        ] {
            assert_eq!(finite(&[value]), Err(SolverError::InvalidSpectrum));
        }
        let a = Complex64::new(2.0, 0.0);
        assert_eq!(
            check_pair(a, Complex64::new(2.0 - 128.0 * f64::EPSILON, 0.0)),
            Ok(())
        );
        assert_eq!(
            check_pair(a, Complex64::new(2.0 - 129.0 * f64::EPSILON, 0.0)),
            Err(SolverError::InvalidSpectrum)
        );
    }
}
