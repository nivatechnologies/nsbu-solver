use crate::{regions::RegionCoverage, v2_experiment::reference::regional::RegionalTrackingSample};
use nsbu_solver::domain::{Layout, TickClock};

/// Actual regional-sampling context retained without interpreting it as volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageSamplingMetadata {
    /// Actual regional tracking lattice, retained without interpreting it as a volume quadrature grid.
    pub layout: Layout,
    /// Number of actual regional tracking sample points.
    pub points: usize,
    /// Sample counts for the exclusive actual `SpatialRegion::Core` class, by branch/quantity; None is `NoSamples`.
    pub sampled_core: [Option<usize>; 24],
    /// Sample counts for the exclusive actual `SpatialRegion::Annulus` class, by branch/quantity; None is `NoSamples`.
    pub sampled_annulus: [Option<usize>; 24],
}

/// Three empirical panel settings for both reviewed nominal regions at one accepted clock.
#[derive(Debug, Clone, Copy)]
pub struct CoverageFamilySample {
    pub(super) clock: TickClock,
    pub(super) family_identity: [u8; 32],
    pub(super) tracking_identity: [u8; 32],
    pub(super) core: [RegionCoverage; 3],
    pub(super) annulus: [RegionCoverage; 3],
    pub(super) sampling: CoverageSamplingMetadata,
}
impl CoverageFamilySample {
    /// Exact accepted clock shared with the bound regional tracking report.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Bound exact-v2 family identity.
    pub fn family_identity(self) -> [u8; 32] {
        self.family_identity
    }
    /// Identity carried by the actual regional tracking report.
    pub fn tracking_identity(self) -> [u8; 32] {
        self.tracking_identity
    }
    /// Raw empirical coverage results for nominal core at all three panel settings.
    pub fn core(self) -> [RegionCoverage; 3] {
        self.core
    }
    /// Raw empirical coverage results for nominal annulus at all three panel settings.
    pub fn annulus(self) -> [RegionCoverage; 3] {
        self.annulus
    }
    /// Actual sampled-class metadata, deliberately separate from nominal coverage.
    pub fn sampling(self) -> CoverageSamplingMetadata {
        self.sampling
    }
    pub(crate) fn from_parts(
        clock: TickClock,
        family_identity: [u8; 32],
        regional: RegionalTrackingSample,
        core: [RegionCoverage; 3],
        annulus: [RegionCoverage; 3],
        sampling: CoverageSamplingMetadata,
    ) -> Self {
        Self {
            clock,
            family_identity,
            tracking_identity: regional.identity(),
            core,
            annulus,
            sampling,
        }
    }
}
