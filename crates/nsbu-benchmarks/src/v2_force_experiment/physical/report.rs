//! Complete physical quantities and pair order for one synchronized force-family clock.
use crate::{
    v2_experiment::physical::PhysicalExtrema, v2_force_experiment::ForceFamilySettings, CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
    SolverError,
};

/// Fixed velocity, ordered gradient, ordered Hessian, and vorticity order.
pub const FORCE_PHYSICAL_QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

/// Permanent interpretation of sampled force-grid physical differences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcePhysicalStatus {
    /// Raw diagnostic evidence without a force-precision or sufficiency decision.
    DiagnosticOnly,
}

/// Two complete force-grid pair findings for one physical quantity.
#[derive(Debug, Clone, Copy)]
pub struct ForcePhysicalQuantity {
    /// Complete physical quantity; tensor entries retain their ordered definition.
    pub quantity: PhysicalQuantity,
    /// M0/M1 followed by M1/M2 sampled physical errors.
    pub pairs: [LocalError; 2],
    extrema: [PhysicalExtrema; 2],
}
impl ForcePhysicalQuantity {
    /// Sampled peak witnesses in M0/M1, M1/M2 order.
    pub fn extrema(self, pair: usize) -> Result<PhysicalExtrema, SolverError> {
        self.extrema
            .get(pair)
            .copied()
            .ok_or(SolverError::InvalidPayload)
    }

    pub(super) fn new(
        quantity: PhysicalQuantity,
        pairs: [LocalError; 2],
        extrema: [PhysicalExtrema; 2],
    ) -> Self {
        Self {
            quantity,
            pairs,
            extrema,
        }
    }
}

/// Transactionally completed physical force-grid report at one actual family clock.
#[derive(Debug, Clone, Copy)]
pub struct ForcePhysicalSample {
    pub(super) clock: TickClock,
    pub(super) identity: [u8; 32],
    pub(super) settings: ForceFamilySettings,
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) quantities: [ForcePhysicalQuantity; 4],
}
impl ForcePhysicalSample {
    /// Exact synchronized clock of all three independently evolved states.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Immutable force-family identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Fixed velocity/integration policy and the three force sampling grids.
    pub fn settings(self) -> ForceFamilySettings {
        self.settings
    }
    /// Common physical sampling lattice used for both raw pairs.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Relative denominator floors in [`FORCE_PHYSICAL_QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Four complete findings in [`FORCE_PHYSICAL_QUANTITIES`] order.
    pub fn quantities(&self) -> &[ForcePhysicalQuantity; 4] {
        &self.quantities
    }
    /// Frozen exact-v2 mathematical case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
    /// This consumer applies no force-precision or sufficiency policy.
    pub fn status(self) -> ForcePhysicalStatus {
        ForcePhysicalStatus::DiagnosticOnly
    }
}
