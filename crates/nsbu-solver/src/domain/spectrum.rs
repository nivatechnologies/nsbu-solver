//! Input validation for finite strict-band Hermitian half spectra.
use super::Layout;
use crate::{Complex64, SolverError};

/// Validate one normalized component without allocating or changing coefficients.
/// Tolerance is an absolute componentwise conjugacy tolerance; Nyquist is exactly zero.
pub fn validate_spectrum(
    layout: Layout,
    values: &[Complex64],
    tolerance: f64,
) -> Result<(), SolverError> {
    if values.len() != layout.half_len() || !tolerance.is_finite() || tolerance < 0.0 {
        return Err(SolverError::InvalidPayload);
    }
    let [nx, ny, nz] = layout.dimensions();
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..=nz / 2 {
                let position = [i, j, k];
                validate_coefficient(layout, values, position, tolerance)?;
            }
        }
    }
    Ok(())
}

fn validate_coefficient(
    layout: Layout,
    values: &[Complex64],
    position: [usize; 3],
    tolerance: f64,
) -> Result<(), SolverError> {
    let [nx, ny, _] = layout.dimensions();
    let [i, j, k] = position;
    let value = values[layout.index(position)?];
    if !value.re.is_finite() || !value.im.is_finite() {
        return Err(SolverError::InvalidSpectrum);
    }
    if layout.is_nyquist(position)? && value != Complex64::new(0.0, 0.0) {
        return Err(SolverError::InvalidSpectrum);
    }
    if k == 0 {
        let partner = values[layout.index([(nx - i) % nx, (ny - j) % ny, 0])?];
        let difference = value - partner.conj();
        if difference.re.abs() > tolerance || difference.im.abs() > tolerance {
            return Err(SolverError::InvalidSpectrum);
        }
    }
    Ok(())
}
