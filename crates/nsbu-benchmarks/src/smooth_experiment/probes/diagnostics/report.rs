//! Privately completed physical findings preserve reconstructed origin and physical clock.
use crate::smooth_experiment::{physical::QuantityRefinement, probes::ProbeSample};
use nsbu_solver::domain::{Layout, TickClock};

/// Six complete reconstructed physical quantities, with all five actual trajectory pairs each.
/// Pressure has one global mean-zero gauge; no regional mean or analytical pressure is substituted.
#[derive(Debug, Clone, Copy)]
pub struct ProbePhysicalSample {
    pub(super) origin: ProbeSample,
    pub(super) samples: Layout,
    pub(super) quantities: [QuantityRefinement; 6],
}
impl ProbePhysicalSample {
    /// Physical probe clock, distinct from actual states at the accepted lookahead endpoints.
    pub fn clock(self) -> TickClock {
        self.origin.clock()
    }
    /// All six accepted-node origins plus the independently reconstructed Fourier comparisons.
    pub fn reconstruction(self) -> ProbeSample {
        self.origin
    }
    /// Common unaligned physical lattice used for all thirty complete comparisons.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Velocity, gradient, Hessian, vorticity, physical pressure and pressure gradient.
    pub fn quantities(&self) -> &[QuantityRefinement; 6] {
        &self.quantities
    }
}
