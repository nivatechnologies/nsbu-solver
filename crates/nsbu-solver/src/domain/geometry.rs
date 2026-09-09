//! Validated periodic geometry with fixed positive viscosity.
use super::Layout;
use crate::SolverError;

/// Immutable physical domain and strict-band retained grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Domain {
    layout: Layout,
    lengths: [f64; 3],
    viscosity: f64,
}

impl Domain {
    /// Require positive finite lengths/viscosity and positive four-multiple grids.
    pub fn new(
        dimensions: [usize; 3],
        lengths: [f64; 3],
        viscosity: f64,
    ) -> Result<Self, SolverError> {
        if dimensions.iter().any(|n| !n.is_multiple_of(4))
            || lengths
                .iter()
                .any(|&length| !length.is_finite() || length <= 0.0)
            || !viscosity.is_finite()
            || viscosity <= 0.0
        {
            return Err(SolverError::InvalidDomain);
        }
        Ok(Self {
            layout: Layout::new(dimensions)?,
            lengths,
            viscosity,
        })
    }

    /// Retained half-spectrum layout.
    pub fn layout(self) -> Layout {
        self.layout
    }

    /// Positive physical side lengths.
    pub fn lengths(self) -> [f64; 3] {
        self.lengths
    }

    /// Fixed positive kinematic viscosity.
    pub fn viscosity(self) -> f64 {
        self.viscosity
    }

    /// Three-halves padded layout, checked before any allocation.
    pub fn padded_layout(self) -> Result<Layout, SolverError> {
        let mut dimensions = self.layout.dimensions();
        for n in &mut dimensions {
            *n = n.checked_mul(3).ok_or(SolverError::SizeOverflow)? / 2;
        }
        Layout::new(dimensions)
    }
}
