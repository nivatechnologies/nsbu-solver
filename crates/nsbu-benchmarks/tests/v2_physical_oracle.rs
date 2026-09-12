//! Independent signed Fourier sums for state and borrowed coefficient comparisons.
#[path = "v2_physical_oracle_core.rs"]
mod core;

pub use core::{view_errors, view_point_magnitudes, Errors};
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};

/// Independently accumulated sampled Euclidean/Frobenius error norms.
#[derive(Debug, Clone, Copy, Default)]
pub struct Norms {
    /// Root mean squared tensor magnitude over all physical grid points.
    pub rms: f64,
    /// Largest tensor magnitude on that same grid.
    pub peak: f64,
}

fn views(state: &SpectralState) -> [&[Complex64]; 3] {
    std::array::from_fn(|axis| state.component(axis).unwrap())
}

/// All four complete tensor norms, with independently indexed modes and physical derivatives.
/// Input domains are the unit-cube family; every fine mode and the mean are retained.
pub fn pair_norms(left: &SpectralState, right: &SpectralState, samples: Layout) -> [Norms; 4] {
    pair_errors(left, right, samples, [1.0; 4]).map(|error| Norms {
        rms: error.rms,
        peak: error.peak,
    })
}

/// All pointwise reductions with explicit relative floors and independent signed sums.
pub fn pair_errors(
    left: &SpectralState,
    right: &SpectralState,
    samples: Layout,
    floors: [f64; 4],
) -> [Errors; 4] {
    view_errors(
        left.plan().domain(),
        views(left),
        right.plan().domain(),
        views(right),
        samples,
        floors,
    )
}

/// Difference and finer-state magnitudes reconstructed at one reported sample witness.
pub fn point_magnitudes(
    left: &SpectralState,
    right: &SpectralState,
    samples: Layout,
    linear: usize,
) -> ([f64; 4], [f64; 4]) {
    view_point_magnitudes(
        left.plan().domain(),
        views(left),
        right.plan().domain(),
        views(right),
        samples,
        linear,
    )
}
