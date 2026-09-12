//! Read-only analytical velocity/derivative tracking for all six exact-v2 family branches.
mod plan;
pub mod regional;
mod report;
pub(crate) mod tracking;
use super::{FamilyError, V2Family};
use crate::fields::reference::ReferenceEvaluation;
use nsbu_solver::{
    diagnostics::{
        derivatives::DerivativeWorkspace, local::LocalError, physical::PhysicalQuantity,
    },
    domain::{SpectralState, TickClock},
    SolverError,
};
pub(crate) use plan::{kernel_reservation, work as tracking_work};
pub use plan::{ReferenceTrackingBounds, ReferenceTrackingPlan, ReferenceTrackingWork};
pub use report::{BranchTracking, ReferenceTrackingSample, TrackingQuantity};

/// Fixed velocity, complete gradient, ordered Hessian and physical curl order.
pub const QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

/// Tracking-specific failures preserve their numerical, reference or family origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReferenceTrackingError {
    /// The borrowed family identity, clock, schedule or terminal state was invalid.
    Family(FamilyError),
    /// Analytical binary64 reference or bounded root evaluation failed.
    Reference(crate::BenchmarkError),
    /// Diagnostic allocation, FFT, spectrum or work admission failed.
    Numerical(SolverError),
    /// A prior numerical/reference computation failed before complete publication.
    Terminated,
}
impl From<FamilyError> for ReferenceTrackingError {
    fn from(error: FamilyError) -> Self {
        Self::Family(error)
    }
}
impl From<crate::BenchmarkError> for ReferenceTrackingError {
    fn from(error: crate::BenchmarkError) -> Self {
        Self::Reference(error)
    }
}
impl From<SolverError> for ReferenceTrackingError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

/// One cached analytical grid and sequential actual-state derivative scratch.
pub struct ReferenceTrackingWorkspace<'a> {
    plan: ReferenceTrackingPlan<'a>,
    derivatives: [DerivativeWorkspace; 3],
    actual: Vec<f64>,
    error_magnitudes: Vec<f64>,
    reference_magnitudes: Vec<f64>,
    references: Vec<ReferenceEvaluation>,
    charged: ReferenceTrackingWork,
    next: usize,
    failed: bool,
}
impl<'a> ReferenceTrackingWorkspace<'a> {
    /// Allocate the jointly admitted consumer without evaluating a reference or state.
    pub fn new(plan: ReferenceTrackingPlan<'a>) -> Result<Self, SolverError> {
        let sources = [0, 1, 2].map(|index| plan.family.branches[index].resources().domain());
        let count = plan.samples.real_len();
        Ok(Self {
            derivatives: tracking::derivatives(sources, plan.samples, plan.bounds.storage_bytes)?,
            actual: tracking::real(count)?,
            error_magnitudes: tracking::real(count)?,
            reference_magnitudes: tracking::real(count)?,
            references: tracking::references(count)?,
            plan,
            charged: ReferenceTrackingWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining admitted report attempts; numerical termination cannot refresh them.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Worst-case work charged for all attempted reports, including refusals.
    pub fn charged_work(&self) -> ReferenceTrackingWork {
        self.charged
    }
    /// Next required exact family clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// True after a numerical computation failed before complete publication.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Measure every branch and quantity before publishing one immutable report.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<ReferenceTrackingSample, ReferenceTrackingError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        match self.compute(family, clock) {
            Ok(sample) => {
                self.next += 1;
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), ReferenceTrackingError> {
        if self.failed {
            return Err(ReferenceTrackingError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.reference_evaluations += self.plan.per_attempt.reference_evaluations;
        self.charged.root_iterations += self.plan.per_attempt.root_iterations;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &V2Family<'_>,
        clock: TickClock,
    ) -> Result<ReferenceTrackingSample, ReferenceTrackingError> {
        self.evaluate_references(clock)?;
        let branches = [
            self.branch(family, 0)?,
            self.branch(family, 1)?,
            self.branch(family, 2)?,
            self.branch(family, 3)?,
            self.branch(family, 4)?,
            self.branch(family, 5)?,
        ];
        Ok(ReferenceTrackingSample {
            clock,
            identity: self.plan.family.identity(),
            samples: self.plan.samples,
            floors: self.plan.floors,
            branches,
        })
    }
    fn evaluate_references(&mut self, clock: TickClock) -> Result<(), ReferenceTrackingError> {
        tracking::evaluate_references(&mut self.references, self.plan.samples, clock)
    }
    fn branch(
        &mut self,
        family: &V2Family<'_>,
        index: usize,
    ) -> Result<BranchTracking, ReferenceTrackingError> {
        let state = family.branches[index].state();
        Ok(BranchTracking {
            branch: index,
            quantities: [
                self.quantity(state, index, PhysicalQuantity::Vector, self.plan.floors[0])?,
                self.quantity(
                    state,
                    index,
                    PhysicalQuantity::Gradient,
                    self.plan.floors[1],
                )?,
                self.quantity(state, index, PhysicalQuantity::Hessian, self.plan.floors[2])?,
                self.quantity(
                    state,
                    index,
                    PhysicalQuantity::Vorticity,
                    self.plan.floors[3],
                )?,
            ],
        })
    }
    fn quantity(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<TrackingQuantity, ReferenceTrackingError> {
        self.prepare_quantity(state, branch, quantity)?;
        Ok(TrackingQuantity {
            quantity,
            error: self.reduce(quantity.components(), floor)?,
        })
    }
    fn prepare_quantity(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
    ) -> Result<(), ReferenceTrackingError> {
        let values = [
            state.component(0)?,
            state.component(1)?,
            state.component(2)?,
        ];
        Ok((tracking::TrackingScratch {
            derivatives: &mut self.derivatives,
            actual: &mut self.actual,
            errors: &mut self.error_magnitudes,
            reference_magnitudes: &mut self.reference_magnitudes,
            references: &self.references,
        })
        .prepare_quantity(values, branch, quantity)?)
    }
    fn reduce(&self, components: usize, floor: f64) -> Result<LocalError, SolverError> {
        tracking::reduce(
            components,
            &self.error_magnitudes,
            &self.reference_magnitudes,
            floor,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::tracking::accumulate_magnitude;

    #[test]
    fn component_magnitudes_preserve_tiny_values_and_avoid_intermediate_square_overflow() {
        let tiny = accumulate_magnitude(0.0, 1e-200).unwrap();
        assert!(tiny > 0.0);
        let large = accumulate_magnitude(1e200, 1e200).unwrap();
        assert!(large.is_finite());
        assert!((large / 1e200 - 2.0_f64.sqrt()).abs() < 1e-15);
        assert!(accumulate_magnitude(f64::MAX, f64::MAX).is_err());
    }
}
