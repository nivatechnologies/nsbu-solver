//! Quantitative spectrum defects without correcting or filtering the supplied coefficients.
use super::squares::finite;
use crate::{domain::Layout, Complex64, SolverError};

/// Measured maxima in normalized coefficient units, separate from physical field norms.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpectrumIntegrity {
    /// Largest coefficient magnitude on any excluded Nyquist hyperplane.
    pub nyquist_max: f64,
    /// Largest conjugacy defect on either self-conjugate third-coordinate plane.
    pub hermitian_max: f64,
    /// Largest imaginary part of the mean coefficient across components.
    pub mean_imaginary_max: f64,
}

/// Inspect finite complete payloads, including spectra that fail the strict-state constraints.
/// This reports defects instead of repairing them. No acceptance decision is made here.
pub fn inspect(
    layout: Layout,
    fields: [&[Complex64]; 3],
) -> Result<SpectrumIntegrity, SolverError> {
    for field in fields {
        if field.len() != layout.half_len() {
            return Err(SolverError::InvalidPayload);
        }
        if field
            .iter()
            .any(|value| !value.re.is_finite() || !value.im.is_finite())
        {
            return Err(SolverError::InvalidSpectrum);
        }
    }
    let mut report = SpectrumIntegrity::default();
    for field in fields {
        component(layout, field, &mut report)?;
        report.mean_imaginary_max = report.mean_imaginary_max.max(field[0].im.abs());
    }
    Ok(report)
}

fn component(
    layout: Layout,
    field: &[Complex64],
    report: &mut SpectrumIntegrity,
) -> Result<(), SolverError> {
    let [nx, ny, nz] = layout.dimensions();
    for (index, &value) in field.iter().enumerate() {
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            report.nyquist_max = report.nyquist_max.max(finite(value.re.hypot(value.im))?);
        }
        let [i, j, k] = position;
        if k == 0 || k == nz / 2 {
            let partner = field[layout.index([(nx - i) % nx, (ny - j) % ny, k])?];
            let defect = value - partner.conj();
            report.hermitian_max = report
                .hermitian_max
                .max(finite(defect.re.hypot(defect.im))?);
        }
    }
    Ok(())
}
