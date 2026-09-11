//! Exact probe clocks and accepted-node origins remain separate from current states.
use nsbu_solver::{
    diagnostics::comparison::BandComparison,
    domain::{Domain, TickClock},
    Complex64,
};

/// Actual accepted macro endpoints used for one branch reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOrigin {
    /// Three accepted nodes, including any lookahead nodes after the probe clock.
    pub accepted_nodes: [TickClock; 3],
    /// Current independently integrated state clock when the sample was produced.
    pub state_clock: TickClock,
}

/// Complete five-pair findings at one common off-stage clock.
#[derive(Debug, Clone, Copy)]
pub struct ProbeSample {
    pub(super) clock: TickClock,
    pub(super) origins: [ProbeOrigin; 6],
    pub(super) values: [BandComparison; 5],
    pub(super) derivatives: [BandComparison; 5],
    pub(super) identity: [u8; 32],
}
impl ProbeSample {
    /// Common physical probe clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Six actual accepted-node histories and current state clocks.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        &self.origins
    }
    /// N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO value comparisons.
    pub fn values(&self) -> &[BandComparison; 5] {
        &self.values
    }
    /// The same five comparisons for reconstructed physical-time derivatives.
    pub fn derivatives(&self) -> &[BandComparison; 5] {
        &self.derivatives
    }
    /// Identity of the immutable exact-v2 family and probe manifest.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
}

/// Read-only reconstructed scratch; it is neither an integrated state nor a checkpoint.
#[derive(Debug)]
pub struct ProbeFields<'a> {
    /// Retained source domain for this branch.
    pub domain: Domain,
    /// Common off-stage physical clock.
    pub clock: TickClock,
    /// Actual accepted history used by the interpolation.
    pub origin: ProbeOrigin,
    /// Reconstructed velocity coefficients.
    pub value: [&'a [Complex64]; 3],
    /// Reconstructed physical-time derivative coefficients.
    pub derivative: [&'a [Complex64]; 3],
}
