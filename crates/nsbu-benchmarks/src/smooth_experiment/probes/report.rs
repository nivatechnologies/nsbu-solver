//! Exact probe clocks and accepted-history origins stay separate from each owner's current state.
use nsbu_solver::{
    diagnostics::comparison::BandComparison,
    domain::{Domain, TickClock},
    Complex64,
};

/// Actual accepted macro endpoints used for one branch's quintic reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOrigin {
    /// Three actual accepted nodes; they can include future nodes relative to the probe.
    pub accepted_nodes: [TickClock; 3],
    /// Actual independently integrated state clock when this sample was produced.
    pub state_clock: TickClock,
}
/// Complete privately constructed value/physical-time-derivative findings at one common probe.
#[derive(Debug, Clone, Copy)]
pub struct ProbeSample {
    pub(super) clock: TickClock,
    pub(super) origins: [ProbeOrigin; 6],
    pub(super) values: [BandComparison; 5],
    pub(super) derivatives: [BandComparison; 5],
}
impl ProbeSample {
    /// Synchronized physical observation time, which may differ from every current state clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Six actual histories; no supplied artifact is promoted to an accepted-state origin.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        &self.origins
    }
    /// Full-band N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO value comparisons.
    pub fn values(&self) -> &[BandComparison; 5] {
        &self.values
    }
    /// The same five comparisons for independently reconstructed physical-time derivatives.
    pub fn derivatives(&self) -> &[BandComparison; 5] {
        &self.derivatives
    }
}
/// Read-only current interpolation scratch; advancing the owner requires ending this borrow.
/// This view is a reconstructed field, never an integrated SpectralState or a checkpoint.
#[derive(Debug)]
pub struct ProbeFields<'a> {
    /// Full retained source domain and unit conventions for this branch.
    pub domain: Domain,
    /// Actual physical probe time.
    pub clock: TickClock,
    /// Exact accepted history and current state clock.
    pub origin: ProbeOrigin,
    /// Complete unaligned reconstructed velocity coefficients.
    pub value: [&'a [Complex64]; 3],
    /// Complete physical-time derivative coefficients.
    pub derivative: [&'a [Complex64]; 3],
}
