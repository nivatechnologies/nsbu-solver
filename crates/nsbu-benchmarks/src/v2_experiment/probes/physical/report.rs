//! Complete five-pair physical findings with their reconstruction provenance.
use crate::{
    v2_experiment::{
        physical::PhysicalExtrema,
        probes::{ProbeOrigin, ProbeSample},
    },
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Domain, Layout, TickClock},
    SolverError,
};

/// Fixed velocity, ordered gradient, ordered Hessian, and vorticity order.
pub const PROBE_PHYSICAL_QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

/// Permanent interpretation of reconstructed sampled physical differences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePhysicalStatus {
    /// Raw diagnostic evidence without an acceptance or interpolation-error decision.
    DiagnosticOnly,
}

/// Five complete branch-pair findings for one physical quantity.
#[derive(Debug, Clone, Copy)]
pub struct ProbePhysicalQuantity {
    /// Complete physical quantity with ordered tensor entries.
    pub quantity: PhysicalQuantity,
    /// N0/N1, N1/N2, H0/H1, H1/H2, and CM/HO errors.
    pub pairs: [LocalError; 5],
    extrema: [PhysicalExtrema; 5],
}
impl ProbePhysicalQuantity {
    /// Peak witnesses in the documented five-pair order.
    pub fn extrema(self, pair: usize) -> Result<PhysicalExtrema, SolverError> {
        self.extrema
            .get(pair)
            .copied()
            .ok_or(SolverError::InvalidPayload)
    }

    pub(super) fn new(
        quantity: PhysicalQuantity,
        pairs: [LocalError; 5],
        extrema: [PhysicalExtrema; 5],
    ) -> Self {
        Self {
            quantity,
            pairs,
            extrema,
        }
    }
}

/// Transactionally complete physical findings at one actual probe publication.
#[derive(Debug, Clone, Copy)]
pub struct ProbePhysicalSample {
    pub(super) reconstruction: ProbeSample,
    pub(super) domains: [Domain; 6],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) quantities: [ProbePhysicalQuantity; 4],
}
impl ProbePhysicalSample {
    /// Exact physical probe clock, distinct from accepted state clocks in its origins.
    pub fn clock(self) -> TickClock {
        self.reconstruction.clock()
    }
    /// Immutable probe-family and manifest identity.
    pub fn identity(self) -> [u8; 32] {
        self.reconstruction.identity()
    }
    /// Original complete raw probe report, including all accepted-node origins.
    pub fn reconstruction(self) -> ProbeSample {
        self.reconstruction
    }
    /// Actual retained source domains in six-branch family order.
    pub fn source_domains(self) -> [Domain; 6] {
        self.domains
    }
    /// Six exact interpolation origins copied from the raw publication.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        self.reconstruction.origins()
    }
    /// Common physical sampling lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Relative denominator floors in [`PROBE_PHYSICAL_QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Four complete findings in [`PROBE_PHYSICAL_QUANTITIES`] order.
    pub fn quantities(&self) -> &[ProbePhysicalQuantity; 4] {
        &self.quantities
    }
    /// Frozen exact-v2 mathematical case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
    /// This consumer applies no acceptance or error-budget policy.
    pub fn status(self) -> ProbePhysicalStatus {
        ProbePhysicalStatus::DiagnosticOnly
    }
}
