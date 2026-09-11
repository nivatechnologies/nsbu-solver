//! Sample locations bind physical maxima to the lattice that produced them.
use nsbu_solver::{diagnostics::physical::PhysicalComparison, domain::Layout, SolverError};

/// A maximum is absent only when its supplied sample stream is empty.
/// Ties retain the first linear index in x-major, z-fast order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleMaximum {
    /// No samples were supplied; absence is never represented as zero error.
    NoSamples,
    /// First sample attaining the finite maximum.
    Measured {
        /// Sampling lattice on which the maximum was found.
        layout: Layout,
        /// Zero-based linear sample index in x-major, z-fast order.
        linear: usize,
        /// Three-dimensional index corresponding to `linear`.
        index: [usize; 3],
        /// Finite sampled magnitude.
        value: f64,
    },
}

/// Absolute-error, relative-error and reference-magnitude maxima for one pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalExtrema {
    /// Largest complete field-difference magnitude.
    pub error: SampleMaximum,
    /// Largest difference divided by the pointwise reference/floor scale.
    pub relative_error: SampleMaximum,
    /// Largest finer/comparison-state magnitude; this is not an analytical reference.
    pub reference: SampleMaximum,
}

impl PhysicalExtrema {
    pub(super) fn from_comparison(
        comparison: &PhysicalComparison<'_>,
        floor: f64,
    ) -> Result<Self, SolverError> {
        let layout = comparison.sample_layout();
        let errors = comparison.error_magnitudes();
        let references = comparison.reference_magnitudes();
        Ok(Self {
            error: maximum(layout, errors.iter().copied())?,
            relative_error: maximum(
                layout,
                errors
                    .iter()
                    .zip(references)
                    .map(|(&error, &reference)| error / reference.max(floor)),
            )?,
            reference: maximum(layout, references.iter().copied())?,
        })
    }
}

fn maximum(
    layout: Layout,
    values: impl IntoIterator<Item = f64>,
) -> Result<SampleMaximum, SolverError> {
    let mut found: Option<(usize, f64)> = None;
    let mut count = 0usize;
    for (linear, value) in values.into_iter().enumerate() {
        if !value.is_finite() || value < 0.0 {
            return Err(SolverError::InvalidPayload);
        }
        if found.is_none_or(|(_, peak)| value > peak) {
            found = Some((linear, value));
        }
        count = linear + 1;
    }
    let Some((linear, value)) = found else {
        return Ok(SampleMaximum::NoSamples);
    };
    if count != layout.real_len() {
        return Err(SolverError::InvalidPayload);
    }
    let [_, ny, nz] = layout.dimensions();
    Ok(SampleMaximum::Measured {
        layout,
        linear,
        index: [linear / (ny * nz), (linear / nz) % ny, linear % nz],
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_invalid_and_first_tied_maxima_remain_distinct() {
        let layout = Layout::new([4; 3]).unwrap();
        assert_eq!(maximum(layout, []).unwrap(), SampleMaximum::NoSamples);
        assert!(maximum(layout, [f64::NAN]).is_err());
        assert!(maximum(layout, [-1.0]).is_err());
        assert!(maximum(layout, std::iter::repeat_n(0.0, 65)).is_err());
        assert_eq!(
            maximum(
                layout,
                (0..64).map(|i| if i == 1 || i == 2 { 2.0 } else { 0.0 })
            )
            .unwrap(),
            SampleMaximum::Measured {
                layout,
                linear: 1,
                index: [0, 0, 1],
                value: 2.0
            }
        );
    }
}
