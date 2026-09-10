//! Complete residual fields are compared directly; differences of norms never substitute for them.
use crate::smooth_experiment::{probes::ProbeSample, residual::ResidualSample};
use nsbu_solver::{
    diagnostics::comparison::BandComparison, domain::TickClock,
    verification::reconstruction::ProbeRefinement,
};

/// Six independently assembled off-stage defects, five full-field differences and actual geometry.
#[derive(Debug, Clone, Copy)]
pub struct ResidualFamilySample {
    pub(super) origin: ProbeSample,
    pub(super) branches: [ResidualSample; 6],
    pub(super) comparisons: [BandComparison; 5],
    pub(super) temporal: ProbeRefinement,
}
impl ResidualFamilySample {
    /// Actual common physical probe clock; current integrated-state times can be later.
    pub fn clock(self) -> TickClock {
        self.origin.clock()
    }
    /// Exact accepted-node origins and the original independently reconstructed velocity comparisons.
    pub fn reconstruction(self) -> ProbeSample {
        self.origin
    }
    /// Actual full doubled-band residual norms and accepted-history geometry for every branch.
    pub fn branches(&self) -> &[ResidualSample; 6] {
        &self.branches
    }
    /// Complete residual coefficient differences for N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO.
    pub fn comparisons(&self) -> &[BandComparison; 5] {
        &self.comparisons
    }
    /// Three strictly nested actual temporal reconstruction histories at this non-stage clock.
    pub fn temporal_geometry(self) -> ProbeRefinement {
        self.temporal
    }
}
