//! Sampling changes compare already complete diagnostic statistics, never replace field differences.
use super::super::physical::QuantityRefinement;
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
    SolverError,
};

/// Absolute changes in diagnostic statistics between successive physical sample grids.
/// These are sampling sensitivities, not spatial trajectory errors or supremum bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingChange {
    /// Change in the RMS of the complete field difference.
    pub rms_error: f64,
    /// Change in the largest sampled complete field difference.
    pub peak_error: f64,
    /// Change in the largest pointwise relative error, with the same fixed floor.
    pub peak_relative_error: f64,
    /// Change in the sampled reference magnitude peak.
    pub reference_peak: f64,
}

/// One physical quantity on three independently sampled lattices.
#[derive(Debug, Clone, Copy)]
pub struct SamplingQuantity {
    pub(super) levels: [QuantityRefinement; 3],
}
impl SamplingQuantity {
    /// Complete physical scalar/vector/tensor being compared.
    pub fn quantity(self) -> PhysicalQuantity {
        self.levels[0].quantity
    }
    /// All original spatial, temporal and method differences on every sample lattice.
    pub fn levels(&self) -> &[QuantityRefinement; 3] {
        &self.levels
    }
    /// Pair order is N0/N1, N1/N2, H0/H1, H1/H2, CM/HO.
    pub fn pair(self, pair: usize) -> Result<[LocalError; 3], SolverError> {
        if pair >= 5 {
            return Err(SolverError::InvalidPayload);
        }
        Ok(self.levels.map(|level| {
            [
                level.space[0],
                level.space[1],
                level.time[0],
                level.time[1],
                level.method,
            ][pair]
        }))
    }
    /// Two absolute changes; no inferred convergence rate or unsupported zero error floor.
    pub fn changes(self, pair: usize) -> Result<[SamplingChange; 2], SolverError> {
        let [a, b, c] = self.pair(pair)?;
        Ok([difference(a, b), difference(b, c)])
    }
}
fn difference(a: LocalError, b: LocalError) -> SamplingChange {
    SamplingChange {
        rms_error: (a.rms_error - b.rms_error).abs(),
        peak_error: (a.peak_error - b.peak_error).abs(),
        peak_relative_error: (a.peak_relative_error - b.peak_relative_error).abs(),
        reference_peak: (a.reference_peak - b.reference_peak).abs(),
    }
}

/// Complete six-quantity, five-pair, three-grid measurement from actual accepted states.
#[derive(Debug, Clone, Copy)]
pub struct SamplingSample {
    pub(super) clock: TickClock,
    pub(super) layouts: [Layout; 3],
    pub(super) quantities: [SamplingQuantity; 6],
}
impl SamplingSample {
    /// One exact synchronized accepted clock for all ninety complete comparisons.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Unshifted, unaligned physical sample grids in increasing resolution order.
    pub fn sample_layouts(self) -> [Layout; 3] {
        self.layouts
    }
    /// Velocity, gradient, Hessian, vorticity, physical pressure and pressure gradient.
    pub fn quantities(&self) -> &[SamplingQuantity; 6] {
        &self.quantities
    }
}
