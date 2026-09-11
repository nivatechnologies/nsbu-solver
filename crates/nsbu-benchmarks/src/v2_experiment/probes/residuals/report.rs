//! Complete residual reports retain reconstruction identity and actual accepted-node geometry.
use super::workspace::ResidualSample;
use crate::v2_experiment::probes::ProbeSample;
use nsbu_solver::{
    diagnostics::comparison::BandComparison,
    domain::{Domain, TickClock},
    verification::reconstruction::ProbeRefinement,
    Complex64,
};

/// Six defects and their five complete doubled-band differences.
#[derive(Debug, Clone, Copy)]
pub struct ResidualFamilySample {
    pub(super) reconstruction: ProbeSample,
    pub(super) branches: [ResidualSample; 6],
    pub(super) comparisons: [BandComparison; 5],
    pub(super) temporal: ProbeRefinement,
}
impl ResidualFamilySample {
    /// Exact common off-stage clock.
    pub fn clock(self) -> TickClock {
        self.reconstruction.clock()
    }
    /// Complete reconstruction report and its immutable identity.
    pub fn reconstruction(self) -> ProbeSample {
        self.reconstruction
    }
    /// Six residual norms and actual node geometries.
    pub fn branches(&self) -> &[ResidualSample; 6] {
        &self.branches
    }
    /// N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO residual comparisons.
    pub fn comparisons(&self) -> &[BandComparison; 5] {
        &self.comparisons
    }
    /// Strict nested temporal reconstruction geometry.
    pub fn temporal_geometry(self) -> ProbeRefinement {
        self.temporal
    }
}

/// Read-only complete doubled-band residual coefficients after successful publication.
#[derive(Debug)]
pub struct ResidualFields<'a> {
    /// Exact off-stage report clock.
    pub clock: TickClock,
    /// Child-specific doubled residual domain.
    pub domain: Domain,
    /// Branch residual coefficients on the complete doubled band.
    pub coefficients: [&'a [Complex64]; 3],
}
