use crate::{
    v2_experiment::{
        probes::{ProbeOrigin, ProbeSample},
        reference::BranchTracking,
    },
    CASE_SHA256,
};
use nsbu_solver::domain::{Domain, Layout, TickClock};

/// Scientific status of reconstructed analytical tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeReferenceStatus {
    /// Raw sampled diagnostics with no acceptance or continuum-error policy.
    DiagnosticOnly,
}

/// Complete analytical tracking of all six reconstructed velocity fields.
#[derive(Debug, Clone, Copy)]
pub struct ProbeReferenceSample {
    pub(super) reconstruction: ProbeSample,
    pub(super) domains: [Domain; 6],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) branches: [BranchTracking; 6],
}

impl ProbeReferenceSample {
    /// Exact physical probe clock, distinct from accepted nodes in its origins.
    pub fn clock(self) -> TickClock {
        self.reconstruction.clock()
    }
    /// Immutable probe-family and manifest identity.
    pub fn identity(self) -> [u8; 32] {
        self.reconstruction.identity()
    }
    /// Complete raw reconstruction report and provenance.
    pub fn reconstruction(self) -> ProbeSample {
        self.reconstruction
    }
    /// Retained source domains in six-branch order.
    pub fn source_domains(self) -> [Domain; 6] {
        self.domains
    }
    /// Six exact accepted-node origins used for reconstruction.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        self.reconstruction.origins()
    }
    /// Common unshifted physical sample lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Relative floors in velocity, gradient, Hessian and vorticity order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Complete six-branch findings.
    pub fn branches(&self) -> &[BranchTracking; 6] {
        &self.branches
    }
    /// Frozen exact-v2 mathematical case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
    /// This consumer applies no acceptance or error-budget policy.
    pub fn status(self) -> ProbeReferenceStatus {
        ProbeReferenceStatus::DiagnosticOnly
    }
}
