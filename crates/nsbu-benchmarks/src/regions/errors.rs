//! Fixed-grid global and regional measurements share every observation without masking failures.
use super::{classify, SpatialRegion};
use crate::{time::BenchmarkTime, BenchmarkError};
use nsbu_solver::{
    diagnostics::local::{ErrorAccumulator, SampledError},
    domain::{Layout, TickClock},
    SolverError,
};

const REGIONS: [SpatialRegion; 5] = [
    SpatialRegion::Core,
    SpatialRegion::Annulus,
    SpatialRegion::InteriorOutsideNominal,
    SpatialRegion::Collar,
    SpatialRegion::Exterior,
];

/// Keep geometry/work refusals distinct from finite measurement failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionalError {
    /// Geometry, clock identity or bounded root work failed.
    Geometry(BenchmarkError),
    /// Field data, capacity or measurement arithmetic failed.
    Measurement(SolverError),
}
impl From<BenchmarkError> for RegionalError {
    fn from(error: BenchmarkError) -> Self {
        Self::Geometry(error)
    }
}
impl From<SolverError> for RegionalError {
    fn from(error: SolverError) -> Self {
        Self::Measurement(error)
    }
}

/// Sampled errors retain global measurements and every spatial class separately.
#[derive(Debug, Clone, Copy)]
pub struct RegionalReport {
    /// Exact physical time shared by all observations.
    pub clock: TickClock,
    /// Uniform unit-periodic sample grid; points are never recentered or aligned.
    pub dimensions: [usize; 3],
    /// True only after every grid point has been supplied successfully.
    pub grid_complete: bool,
    /// All observations, including collar and exterior samples.
    pub global: SampledError,
    /// Explicit labels preserve no-sample states; none implies geometric emptiness.
    pub regions: [(SpatialRegion, SampledError); 5],
    /// Conservative root-iteration budget charged, including unsuccessful classifications.
    pub root_work_charged: usize,
}

/// Fixed-storage collector for one declared grid and time; independent of integrated state.
#[derive(Debug)]
pub struct RegionalErrors {
    clock: TickClock,
    layout: Layout,
    root_budget: usize,
    attempts_left: usize,
    charged: usize,
    next: usize,
    global: ErrorAccumulator,
    regions: [ErrorAccumulator; 5],
}
impl RegionalErrors {
    /// Preflight all grid samples and a finite classification-attempt allowance.
    /// Failed classifications consume an attempt; retries cannot escape the work bound.
    pub fn new(
        clock: TickClock,
        layout: Layout,
        root_budget: usize,
        maximum_attempts: usize,
        relative_floor: f64,
    ) -> Result<Self, RegionalError> {
        BenchmarkTime::new(clock)?;
        if !(1..=128).contains(&root_budget) {
            return Err(BenchmarkError::InvalidInput.into());
        }
        if maximum_attempts < layout.real_len()
            || maximum_attempts.checked_mul(root_budget).is_none()
        {
            return Err(BenchmarkError::DiagnosticWorkExceeded.into());
        }
        let global = ErrorAccumulator::new(layout.real_len(), relative_floor)?;
        Ok(Self {
            clock,
            layout,
            root_budget,
            attempts_left: maximum_attempts,
            charged: 0,
            next: 0,
            global,
            regions: [global; 5],
        })
    }

    /// Unit-periodic point for the next pair of field evaluations, or None after the full grid.
    /// Returning coordinates lets an independent reference use the identical unaligned samples.
    pub fn next_point(&self) -> Option<[f64; 3]> {
        if self.next == self.layout.real_len() {
            return None;
        }
        let [nx, ny, nz] = self.layout.dimensions();
        Some([
            (self.next / (ny * nz)) as f64 / nx as f64,
            ((self.next / nz) % ny) as f64 / ny as f64,
            (self.next % nz) as f64 / nz as f64,
        ])
    }

    /// Consume the next lexicographic grid point (z fastest), with no skipped points.
    /// Failures preserve observations and position; attempted root work stays charged.
    pub fn push(&mut self, actual: [f64; 3], reference: [f64; 3]) -> Result<(), RegionalError> {
        if self.attempts_left == 0 {
            return Err(BenchmarkError::DiagnosticWorkExceeded.into());
        }
        let point = self.next_point().ok_or(SolverError::ResourceLimit)?;
        let mut global = self.global;
        global.push(actual, reference)?;
        self.attempts_left -= 1;
        self.charged += self.root_budget;
        let spatial = classify(point, self.clock, self.root_budget)?.spatial;
        let index = match spatial {
            SpatialRegion::Core => 0,
            SpatialRegion::Annulus => 1,
            SpatialRegion::InteriorOutsideNominal => 2,
            SpatialRegion::Collar => 3,
            SpatialRegion::Exterior => 4,
        };
        let mut region = self.regions[index];
        region.push(actual, reference)?;
        self.global = global;
        self.regions[index] = region;
        self.next += 1;
        Ok(())
    }

    /// Snapshot measurements without accepting a window or inferring empty-region geometry.
    pub fn report(&self) -> Result<RegionalReport, SolverError> {
        let mut regions = REGIONS.map(|region| (region, SampledError::NoSamples));
        for (entry, samples) in regions.iter_mut().zip(self.regions) {
            entry.1 = samples.finish()?;
        }
        Ok(RegionalReport {
            clock: self.clock,
            dimensions: self.layout.dimensions(),
            grid_complete: self.next == self.layout.real_len(),
            global: self.global.finish()?,
            regions,
            root_work_charged: self.charged,
        })
    }
}
