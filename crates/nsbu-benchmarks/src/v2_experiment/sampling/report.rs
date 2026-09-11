//! Complete three-lattice statistics and their sampling sensitivity.
use super::super::physical::{PhysicalExtrema, QuantityRefinement};
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
    SolverError,
};

/// Absolute change between statistics measured on successive sample lattices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingChange {
    /// Change in sampled RMS error.
    pub rms_error: f64,
    /// Change in sampled absolute peak error.
    pub peak_error: f64,
    /// Change in sampled relative peak error.
    pub peak_relative_error: f64,
    /// Change in sampled reference peak.
    pub reference_peak: f64,
}

/// One physical quantity on all three independently sampled lattices.
#[derive(Debug, Clone, Copy)]
pub struct SamplingQuantity {
    pub(super) levels: [QuantityRefinement; 3],
}
impl SamplingQuantity {
    /// Complete vector or ordered tensor quantity.
    pub fn quantity(self) -> PhysicalQuantity {
        self.levels[0].quantity
    }
    /// All raw five-pair metrics at each of the three sample resolutions.
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
    /// Layout-bound peak witnesses for the requested pair on all three levels.
    pub fn extrema(self, pair: usize) -> Result<[PhysicalExtrema; 3], SolverError> {
        let values = self.levels.map(|level| level.extrema(pair));
        Ok([values[0]?, values[1]?, values[2]?])
    }
    /// Two absolute statistic changes, without an inferred rate or monotonicity claim.
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

/// Four quantities, five pairs and three full sample-grid reports at one accepted clock.
#[derive(Debug, Clone, Copy)]
pub struct SamplingSample {
    pub(super) clock: TickClock,
    pub(super) identity: [u8; 32],
    pub(super) layouts: [Layout; 3],
    pub(super) floors: [f64; 4],
    pub(super) quantities: [SamplingQuantity; 4],
}
impl SamplingSample {
    /// Exact synchronized accepted clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Identity of the unchanged actual V2 family.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Strictly increasing unshifted sample lattices.
    pub fn sample_layouts(self) -> [Layout; 3] {
        self.layouts
    }
    /// Fixed velocity/gradient/Hessian/vorticity floors.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Complete retained quantity reports.
    pub fn quantities(&self) -> &[SamplingQuantity; 4] {
        &self.quantities
    }
}
