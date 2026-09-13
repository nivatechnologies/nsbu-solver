//! Bounded small CPU FFT backends with explicit normalization and owned scratch.
mod api;
mod avx;
mod owned;
mod transform;
mod workspace;

use crate::{domain::Layout, Complex64, SolverError};
pub use avx::FftCatalog;

const TRANSVERSE_TILE_LANES: usize = 8;

/// Immutable arithmetic/backend identity for every scalar transform owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FftBackend {
    /// Project-owned mixed-radix binary64 arithmetic; the default compatibility profile.
    OwnedRadix,
    /// RustFFT 6.4.1's explicit x86_64 AVX/FMA planner on a closed length set.
    RustFft6_4_1AvxFma,
}

impl FftBackend {
    /// Refuse an unavailable execution backend without planning or allocating storage.
    pub fn ensure_available(self) -> Result<(), SolverError> {
        match self {
            Self::OwnedRadix => Ok(()),
            Self::RustFft6_4_1AvxFma => avx::ensure_available(),
        }
    }
}

enum BackendPlan {
    Owned([Vec<Complex64>; 3]),
    Avx(Box<[avx::Axes]>),
}

/// Immutable plan for one explicitly selected scalar-transform backend.
pub struct FftPlan {
    layout: Layout,
    backend: BackendPlan,
}

/// Mutable storage belonging to one scalar transform stream, never shared.
#[derive(Debug)]
pub struct FftWorkspace {
    layout: Layout,
    grid: Vec<Complex64>,
    input: Vec<Complex64>,
    output: Vec<Complex64>,
    scratch: Vec<Complex64>,
    transverse_tile: Vec<Complex64>,
}

impl std::fmt::Debug for FftPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FftPlan")
            .field("layout", &self.layout)
            .field("backend", &self.backend())
            .finish()
    }
}
