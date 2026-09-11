//! Completed physical reports retain statistics and their sampled peak locations.
use super::PhysicalExtrema;
use nsbu_solver::{
    diagnostics::{local::LocalError, physical::PhysicalQuantity},
    domain::{Layout, TickClock},
    SolverError,
};

/// Five complete pair findings for one physical quantity.
#[derive(Debug, Clone, Copy)]
pub struct QuantityRefinement {
    /// Measured physical quantity.
    pub quantity: PhysicalQuantity,
    /// Spatial pairs `(N0,N1)` and `(N1,N2)`.
    pub space: [LocalError; 2],
    /// Temporal pairs `(H0,H1)` and `(H1,H2)`.
    pub time: [LocalError; 2],
    /// Finest-grid CM/HO pair.
    pub method: LocalError,
    pub(super) extrema: [PhysicalExtrema; 5],
}
impl QuantityRefinement {
    /// Peak witnesses in spatial, temporal and method pair order.
    pub fn extrema(self, pair: usize) -> Result<PhysicalExtrema, SolverError> {
        self.extrema
            .get(pair)
            .copied()
            .ok_or(SolverError::InvalidPayload)
    }
}

/// Complete physical findings at one actual accepted V2 clock.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalRefinementSample {
    pub(super) clock: TickClock,
    pub(super) identity: [u8; 32],
    pub(super) samples: Layout,
    pub(super) floors: [f64; 4],
    pub(super) quantities: [QuantityRefinement; 4],
}
impl PhysicalRefinementSample {
    /// Actual synchronized accepted clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Identity of the V2 family that produced this report.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Diagnostic physical sample layout.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed relative-error floors in [`super::QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Four findings in [`super::QUANTITIES`] order.
    pub fn quantities(&self) -> &[QuantityRefinement; 4] {
        &self.quantities
    }
}
