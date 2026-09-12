//! Exact-v2 diagnostic geometry; floating masks and quadrature are empirical measurements.
mod coverage;
mod errors;
pub mod physical;
mod point;
use crate::BenchmarkError;
pub(crate) use coverage::status_at as coverage_status_at;
pub use coverage::{CoveragePlan, CoverageStatus, RegionCoverage};
pub use errors::{RegionalError, RegionalErrors, RegionalReport, RegionalTensorErrors};
pub use point::{classify, PointRegion, SpatialRegion, StartupPhase};

const INNER_RADIUS_SQUARED: f64 = 9.0 / 100.0;

/// Nominal radial range in X=(x²+y²)/(2q), on the fixed |eta|<=1/2 interval.
#[derive(Debug, Clone, Copy)]
pub struct NominalRegion {
    low: f64,
    high: f64,
}
impl NominalRegion {
    /// Reviewed nominal core, 0<=X<=1/2.
    pub const CORE: Self = Self {
        low: 0.0,
        high: 0.5,
    };
    /// Reviewed nominal annulus, 1/2<X<=8.
    pub const ANNULUS: Self = Self {
        low: 0.5,
        high: 8.0,
    };

    /// Additional diagnostic regions do not replace the required core/annulus measurements.
    pub fn new(low: f64, high: f64) -> Result<Self, BenchmarkError> {
        if !low.is_finite() || !high.is_finite() || low < 0.0 || high <= low {
            return Err(BenchmarkError::InvalidInput);
        }
        Ok(Self { low, high })
    }
    /// Declared radial bounds, unaffected by floating underflow of cutoff values.
    pub fn bounds(self) -> [f64; 2] {
        [self.low, self.high]
    }
}
