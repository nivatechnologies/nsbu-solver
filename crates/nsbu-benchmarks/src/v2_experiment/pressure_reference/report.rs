//! Immutable complete publication for one accepted family clock.
use super::{ImportedGauge, PressureReferenceWork};
use nsbu_solver::{
    diagnostics::local::LocalError,
    domain::{Layout, TickClock},
};

/// Analytical pressure and gradient error for one actual branch.
#[derive(Debug, Clone, Copy)]
pub struct BranchPressureTracking {
    /// Stable family branch index.
    pub branch: usize,
    /// Globally gauged scalar-pressure error.
    pub pressure: LocalError,
    /// Gauge-invariant three-component pressure-gradient error.
    pub pressure_gradient: LocalError,
}
/// Complete all-six analytical pressure result for one accepted clock.
#[derive(Debug, Clone, Copy)]
pub struct PressureReferenceSample<'a> {
    pub(super) clock: TickClock,
    pub(super) family_identity: [u8; 32],
    pub(super) sample_layout: Layout,
    pub(super) force_layout: Layout,
    pub(super) gauge: ImportedGauge<'a>,
    pub(super) branches: [BranchPressureTracking; 6],
    pub(super) charged: PressureReferenceWork,
}
impl<'a> PressureReferenceSample<'a> {
    /// Actual synchronized accepted family clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Source exact-v2 family identity.
    pub fn family_identity(self) -> [u8; 32] {
        self.family_identity
    }
    /// Shared numerical and analytical physical sample layout.
    pub fn sample_layout(self) -> Layout {
        self.sample_layout
    }
    /// Full doubled pressure/force layout.
    pub fn force_layout(self) -> Layout {
        self.force_layout
    }
    /// Exact imported empirical gauge and authoritative artifact bytes.
    pub fn gauge(self) -> ImportedGauge<'a> {
        self.gauge
    }
    /// Six branch results in family order.
    pub fn branches(&self) -> &[BranchPressureTracking; 6] {
        &self.branches
    }
    /// Cumulative conservative work charged through this publication.
    pub fn charged_work(self) -> PressureReferenceWork {
        self.charged
    }
}
