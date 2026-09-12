//! Project-owned mixed-radix plan construction and reservation accounting.
use super::{BackendPlan, FftPlan, FftWorkspace};
use crate::storage::filled;
use crate::{domain::Layout, Complex64, SolverError};

pub(super) fn reservation(layout: Layout) -> Result<usize, SolverError> {
    let dimensions = layout.dimensions();
    validate_dimensions(dimensions)?;
    let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
    let elements = layout
        .half_len()
        .checked_add(3 * maximum + dimensions.iter().sum::<usize>())
        .ok_or(SolverError::SizeOverflow)?;
    elements
        .checked_mul(16)
        .and_then(|value| {
            value.checked_add(std::mem::size_of::<FftPlan>() + std::mem::size_of::<FftWorkspace>())
        })
        .ok_or(SolverError::SizeOverflow)
}

pub(super) fn new(layout: Layout) -> Result<(FftPlan, FftWorkspace), SolverError> {
    let dimensions = layout.dimensions();
    let mut roots = filled(dimensions.iter().sum(), Complex64::new(0.0, 0.0))?;
    let mut offset = 0;
    for length in dimensions {
        roots[offset..offset + length].copy_from_slice(&roots_for_length(length)?);
        offset += length;
    }
    let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
    let workspace = FftWorkspace {
        layout,
        grid: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        input: filled(maximum, Complex64::new(0.0, 0.0))?,
        output: filled(maximum, Complex64::new(0.0, 0.0))?,
        scratch: filled(maximum, Complex64::new(0.0, 0.0))?,
    };
    Ok((
        FftPlan {
            layout,
            backend: BackendPlan::Owned(roots),
            _layout_compatibility: [0; 48],
        },
        workspace,
    ))
}

pub(super) fn roots(dimensions: [usize; 3], values: &[Complex64], axis: usize) -> &[Complex64] {
    let start = dimensions[..axis].iter().sum::<usize>();
    &values[start..start + dimensions[axis]]
}

fn validate_dimensions(dimensions: [usize; 3]) -> Result<(), SolverError> {
    for n in dimensions {
        let mut factor = n;
        for radix in [2, 3] {
            for _ in 0..usize::BITS {
                if !factor.is_multiple_of(radix) {
                    break;
                }
                factor /= radix;
            }
        }
        if factor != 1 || n > 1024 {
            return Err(SolverError::InvalidDomain);
        }
    }
    Ok(())
}

fn roots_for_length(length: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = filled(length, Complex64::new(0.0, 0.0))?;
    for (index, value) in values.iter_mut().enumerate() {
        let angle = -std::f64::consts::TAU * index as f64 / length as f64;
        *value = Complex64::new(angle.cos(), angle.sin());
    }
    Ok(values)
}
