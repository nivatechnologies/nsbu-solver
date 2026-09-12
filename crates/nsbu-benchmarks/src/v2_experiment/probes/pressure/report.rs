//! Complete off-stage pressure findings with reconstruction provenance.
use crate::{
    v2_experiment::probes::{ProbeOrigin, ProbeSample},
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Domain, Layout, TickClock},
};

/// Pressure or pressure-gradient findings for all five branch pairs.
#[derive(Debug, Clone, Copy)]
pub struct ProbePressureQuantity {
    /// Scalar pressure or its complete three-component gradient.
    pub quantity: PhysicalQuantity,
    /// N0/N1, N1/N2, H0/H1, H1/H2, and CM/HO errors.
    pub pairs: [LocalError; 5],
}
/// Transactionally complete pressure findings at one actual probe clock.
#[derive(Debug, Clone, Copy)]
pub struct ProbePressureSample {
    pub(super) reconstruction: ProbeSample,
    pub(super) source: Domain,
    pub(super) diagnostic: Domain,
    pub(super) samples: Layout,
    pub(super) force_workers: usize,
    pub(super) floors: [f64; 2],
    pub(super) quantities: [ProbePressureQuantity; 2],
}
impl ProbePressureSample {
    /// Exact reconstructed manifest clock measured by this report.
    pub fn clock(self) -> TickClock {
        self.reconstruction.clock()
    }
    /// Identity of the exact probe family and manifest.
    pub fn identity(self) -> [u8; 32] {
        self.reconstruction.identity()
    }
    /// Original complete probe publication used as pressure input.
    pub fn reconstruction(self) -> ProbeSample {
        self.reconstruction
    }
    /// Actual three-node reconstruction origins for all six branches.
    pub fn origins(&self) -> &[ProbeOrigin; 6] {
        self.reconstruction.origins()
    }
    /// Finest retained velocity source domain.
    pub fn source_domain(self) -> Domain {
        self.source
    }
    /// Common doubled pressure-coefficient and force-sampling layout.
    pub fn force_layout(self) -> Layout {
        self.diagnostic.layout()
    }
    /// Physical lattice on which pressure differences were reduced.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Effective original-force worker count; zero denotes serial sampling.
    pub fn force_workers(self) -> usize {
        self.force_workers
    }
    /// Denominator floors in scalar-pressure, pressure-gradient order.
    pub fn relative_floors(self) -> [f64; 2] {
        self.floors
    }
    /// Complete pressure and gradient findings for all five pairs.
    pub fn quantities(&self) -> &[ProbePressureQuantity; 2] {
        &self.quantities
    }
    /// Frozen exact-v2 mathematical case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
}
