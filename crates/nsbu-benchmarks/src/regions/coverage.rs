//! Separately refined, bounded volume-fraction quadrature on the reviewed eta interval.
use super::{NominalRegion, INNER_RADIUS_SQUARED};
use crate::{time::BenchmarkTime, BenchmarkError};
use nsbu_solver::domain::TickClock;

/// Geometry status is independent of whether any spatial grid point samples the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageStatus {
    /// Positive nominal-volume intersection with c_x=1.
    Nonempty,
    /// The closed-form maximum radial limit gives zero-volume intersection.
    RegionEmpty,
}

/// Empirical quadrature evidence, not a certified enclosure of the geometric volume.
#[derive(Debug, Clone, Copy)]
pub struct RegionCoverage {
    /// Geometric status, never inferred from a missing sample or underflowed cutoff.
    pub status: CoverageStatus,
    /// Fraction of nominal physical volume inside the declared c_x=1 sphere.
    pub fraction: f64,
    /// Absolute change between the two independently evaluated panel counts.
    pub refinement_change: f64,
    /// Finer composite Simpson panel count on [-1/2,1/2].
    pub panels: usize,
    /// Actual number of geometry evaluations across both quadratures.
    pub evaluations: usize,
}

/// Fixed-storage quadrature preflight. This profile supports at most 2^20 coarse panels.
#[derive(Debug, Clone, Copy)]
pub struct CoveragePlan {
    panels: usize,
}
impl CoveragePlan {
    /// Require a positive even panel count and enough allowance for both p and 2p grids.
    pub fn new(panels: usize, maximum_evaluations: usize) -> Result<Self, BenchmarkError> {
        if panels < 2 || !panels.is_multiple_of(2) {
            return Err(BenchmarkError::InvalidInput);
        }
        if panels > 1 << 20 {
            return Err(BenchmarkError::DiagnosticWorkExceeded);
        }
        // The supported panel cap keeps 3p+2 addressable and every node index exact in binary64.
        if 3 * panels + 2 > maximum_evaluations {
            return Err(BenchmarkError::DiagnosticWorkExceeded);
        }
        Ok(Self { panels })
    }
    /// Measure geometric coverage independently of local sample counts or field values.
    pub fn evaluate(
        self,
        clock: TickClock,
        region: NominalRegion,
    ) -> Result<RegionCoverage, BenchmarkError> {
        let tau = BenchmarkTime::new(clock)?.remaining();
        let (coarse, coarse_work) = integral(tau, region, self.panels);
        let (fine, fine_work) = integral(tau, region, 2 * self.panels);
        let status = if INNER_RADIUS_SQUARED / (2.0 * tau) <= region.bounds()[0] {
            CoverageStatus::RegionEmpty
        } else {
            CoverageStatus::Nonempty
        };
        Ok(RegionCoverage {
            status,
            fraction: fine,
            refinement_change: (fine - coarse).abs(),
            panels: 2 * self.panels,
            evaluations: coarse_work + fine_work,
        })
    }
}

fn integral(tau: f64, region: NominalRegion, panels: usize) -> (f64, usize) {
    let [low, high] = region.bounds();
    let width = high - low;
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    let mut evaluations = 0;
    for index in 0..=panels {
        let eta = -0.5 + index as f64 / panels as f64;
        let complement = 1.0 - eta * eta;
        let q = tau / complement;
        let z = eta * q.powf(3.0 / 8.0);
        let limit = (INNER_RADIUS_SQUARED - z * z) / (2.0 * q);
        // Cancel the common tau^(11/8) scale and radial width before reduction.
        // Relative weights stay O(1), including the smallest supported remaining tick.
        let weight = complement.powf(-19.0 / 8.0) * (1.0 - eta * eta / 4.0);
        let coefficient = if index == 0 || index == panels {
            1.0
        } else if index.is_multiple_of(2) {
            2.0
        } else {
            4.0
        };
        let share = (limit - low).max(0.0).min(width) / width;
        numerator += coefficient * weight * share;
        denominator += coefficient * weight;
        evaluations += 1;
    }
    (numerator / denominator, evaluations)
}
