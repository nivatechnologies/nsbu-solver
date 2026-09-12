use crate::{
    v2_experiment::{
        probes::{reference::ProbeReferenceSample, ProbeOrigin, ProbeSample},
        reference::regional::RegionalTrackingQuantity,
    },
    CASE_SHA256,
};
use nsbu_solver::domain::{Domain, Layout, TickClock};

/// Scientific interpretation of reconstructed regional analytical errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeRegionalStatus {
    /// Sampled diagnostics without an acceptance or continuum-error decision.
    DiagnosticOnly,
}

/// Four global and five-class regional findings for one reconstructed branch.
#[derive(Debug, Clone, Copy)]
pub struct ProbeRegionalBranch {
    /// Fixed exact-v2 branch slot.
    pub branch: usize,
    /// Velocity, gradient, ordered Hessian and vorticity findings.
    pub quantities: [RegionalTrackingQuantity; 4],
}

/// Complete six-branch regional analytical report at one probe publication.
#[derive(Debug, Clone, Copy)]
pub struct ProbeRegionalSample {
    pub(super) reference: ProbeReferenceSample,
    pub(super) branches: [ProbeRegionalBranch; 6],
}

impl ProbeRegionalSample {
    /// Exact physical probe clock.
    pub fn clock(self) -> TickClock {
        self.reference.clock()
    }
    /// Immutable probe-family and manifest identity.
    pub fn identity(self) -> [u8; 32] {
        self.reference.identity()
    }
    /// Raw reconstruction publication and accepted-node origins.
    pub fn reconstruction(self) -> ProbeSample {
        self.reference.reconstruction()
    }
    /// Analytical global report independently reproduced by this consumer.
    pub fn reference(self) -> ProbeReferenceSample {
        self.reference
    }
    /// Retained source domains in six-branch order.
    pub fn source_domains(self) -> [Domain; 6] {
        self.reference.source_domains()
    }
    /// Six exact accepted-node reconstruction origins.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        self.reference.origins()
    }
    /// Common unshifted physical sampling lattice.
    pub fn sample_layout(self) -> Layout {
        self.reference.sample_layout()
    }
    /// Relative floors in velocity, gradient, Hessian and vorticity order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.reference.relative_floors()
    }
    /// Complete six-branch sampled-region findings.
    pub fn branches(&self) -> &[ProbeRegionalBranch; 6] {
        &self.branches
    }
    /// Frozen exact-v2 mathematical case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
    /// This report does not populate pairwise refinement or geometric coverage channels.
    pub fn status(self) -> ProbeRegionalStatus {
        ProbeRegionalStatus::DiagnosticOnly
    }
}
