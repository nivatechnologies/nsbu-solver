//! Complete global and sampled-region analytical tracking reports.
use crate::regions::RegionalReport;
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
};

/// One complete quantity with its identical global and five-class regional reductions.
#[derive(Debug, Clone, Copy)]
pub struct RegionalTrackingQuantity {
    /// Velocity, gradient, ordered Hessian or vorticity.
    pub quantity: PhysicalQuantity,
    /// Global result from the unchanged analytical tracking reduction.
    pub global: LocalError,
    /// Global plus Core/Annulus/interior/collar/exterior sampled-region results.
    pub regional: RegionalReport,
}

/// Four regional findings for one independently evolved branch.
#[derive(Debug, Clone, Copy)]
pub struct RegionalBranchTracking {
    /// Fixed exact-v2 family branch slot.
    pub branch: usize,
    /// Four findings in [`super::super::QUANTITIES`] order.
    pub quantities: [RegionalTrackingQuantity; 4],
}

/// One synchronized six-branch regional analytical tracking report.
#[derive(Debug, Clone, Copy)]
pub struct RegionalTrackingSample {
    pub(super) clock: TickClock,
    pub(super) identity: [u8; 32],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) branches: [RegionalBranchTracking; 6],
}
impl RegionalTrackingSample {
    /// Exact accepted family clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Immutable family identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Unshifted physical sample lattice.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed floors in [`super::super::QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Every independently evolved branch and complete quantity.
    pub fn branches(&self) -> &[RegionalBranchTracking; 6] {
        &self.branches
    }
}
