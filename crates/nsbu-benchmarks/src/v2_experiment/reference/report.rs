//! Complete per-branch analytical tracking reports.
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
};

/// One quantity's actual-state error against the analytical exact-v2 field.
#[derive(Debug, Clone, Copy)]
pub struct TrackingQuantity {
    /// Velocity, gradient, Hessian or vorticity.
    pub quantity: PhysicalQuantity,
    /// Complete global sampled error with its explicit relative floor.
    pub error: LocalError,
}

/// Four complete tracking quantities for one independently evolved branch.
#[derive(Debug, Clone, Copy)]
pub struct BranchTracking {
    /// Fixed family branch slot.
    pub branch: usize,
    /// Four findings in [`super::QUANTITIES`] order.
    pub quantities: [TrackingQuantity; 4],
}

/// One complete six-branch analytical tracking report at an accepted exact clock.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceTrackingSample {
    pub(super) clock: TickClock,
    pub(super) identity: [u8; 32],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) branches: [BranchTracking; 6],
}
impl ReferenceTrackingSample {
    /// Synchronized accepted clock of every actual state and reference evaluation.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Immutable exact-v2 family identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Unshifted physical sample lattice used for actual and analytical fields.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed floors in [`super::QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// All six independently evolved branch findings.
    pub fn branches(&self) -> &[BranchTracking; 6] {
        &self.branches
    }
}
