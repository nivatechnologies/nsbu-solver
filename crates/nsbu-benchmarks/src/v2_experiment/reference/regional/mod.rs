//! Direct regional reduction of actual-versus-analytical exact-v2 tracking magnitudes.
mod plan;
mod report;
use super::{ReferenceTrackingError, ReferenceTrackingWorkspace, QUANTITIES};
use crate::{
    regions::{RegionalError, RegionalTensorErrors},
    v2_experiment::{FamilyError, V2Family},
};
use nsbu_solver::{
    diagnostics::{local::SampledError, physical::PhysicalQuantity},
    domain::SpectralState,
    SolverError,
};
pub use plan::{RegionalTrackingBounds, RegionalTrackingPlan, RegionalTrackingWork};
pub use report::{RegionalBranchTracking, RegionalTrackingQuantity, RegionalTrackingSample};

/// Regional failures retain the underlying tracking or geometric origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegionalTrackingError {
    /// Family, reference, FFT, spectrum or tracking-attempt failure.
    Tracking(ReferenceTrackingError),
    /// Regional geometry, classification or measurement failure.
    Regional(RegionalError),
}
impl From<ReferenceTrackingError> for RegionalTrackingError {
    fn from(error: ReferenceTrackingError) -> Self {
        Self::Tracking(error)
    }
}
impl From<FamilyError> for RegionalTrackingError {
    fn from(error: FamilyError) -> Self {
        Self::Tracking(error.into())
    }
}
impl From<RegionalError> for RegionalTrackingError {
    fn from(error: RegionalError) -> Self {
        Self::Regional(error)
    }
}
impl From<SolverError> for RegionalTrackingError {
    fn from(error: SolverError) -> Self {
        Self::Tracking(error.into())
    }
}

/// Separately admitted regional path using the unchanged analytical tracking scratch.
pub struct RegionalTrackingWorkspace<'a> {
    plan: RegionalTrackingPlan<'a>,
    tracking: ReferenceTrackingWorkspace<'a>,
    charged: RegionalTrackingWork,
}
impl<'a> RegionalTrackingWorkspace<'a> {
    /// Allocate the analytical tracking owner after joint regional admission.
    pub fn new(plan: RegionalTrackingPlan<'a>) -> Result<Self, SolverError> {
        Ok(Self {
            tracking: ReferenceTrackingWorkspace::new(plan.tracking)?,
            plan,
            charged: RegionalTrackingWork::default(),
        })
    }
    /// Remaining shared report attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.regional_work.attempts - self.charged.attempts
    }
    /// Underlying analytical tracking charges, including refused requests.
    pub fn tracking_work(&self) -> super::ReferenceTrackingWork {
        self.tracking.charged_work()
    }
    /// Added regional classification charges, including refused requests.
    pub fn regional_work(&self) -> RegionalTrackingWork {
        self.charged
    }
    /// Next required exact family clock.
    pub fn next_time(&self) -> Option<nsbu_solver::domain::TickClock> {
        self.tracking.next_time()
    }
    /// Compute all 24 global/regional findings before advancing the report schedule.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<RegionalTrackingSample, RegionalTrackingError> {
        self.charge()?;
        let next = self.tracking.next;
        let clock = self.tracking.plan.family.require_sample(family, next)?;
        match self.compute(family, clock) {
            Ok(sample) => {
                self.tracking.next += 1;
                Ok(sample)
            }
            Err(error) => {
                self.tracking.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), RegionalTrackingError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.tracking.charge()?;
        self.charged.attempts += 1;
        self.charged.classifications += self.plan.per_attempt.classifications;
        self.charged.root_iterations += self.plan.per_attempt.root_iterations;
        self.charged.magnitude_visits += self.plan.per_attempt.magnitude_visits;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &V2Family<'_>,
        clock: nsbu_solver::domain::TickClock,
    ) -> Result<RegionalTrackingSample, RegionalTrackingError> {
        self.tracking.evaluate_references(clock)?;
        let branches = [
            self.branch(family, 0, clock)?,
            self.branch(family, 1, clock)?,
            self.branch(family, 2, clock)?,
            self.branch(family, 3, clock)?,
            self.branch(family, 4, clock)?,
            self.branch(family, 5, clock)?,
        ];
        Ok(RegionalTrackingSample {
            clock,
            identity: self.tracking.plan.family.identity(),
            samples: self.tracking.plan.samples,
            floors: self.tracking.plan.floors,
            branches,
        })
    }
    fn branch(
        &mut self,
        family: &V2Family<'_>,
        branch: usize,
        clock: nsbu_solver::domain::TickClock,
    ) -> Result<RegionalBranchTracking, RegionalTrackingError> {
        let state = family.branches[branch].state();
        Ok(RegionalBranchTracking {
            branch,
            quantities: [
                self.quantity(state, branch, QUANTITIES[0], 0, clock)?,
                self.quantity(state, branch, QUANTITIES[1], 1, clock)?,
                self.quantity(state, branch, QUANTITIES[2], 2, clock)?,
                self.quantity(state, branch, QUANTITIES[3], 3, clock)?,
            ],
        })
    }
    fn quantity(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
        floor: usize,
        clock: nsbu_solver::domain::TickClock,
    ) -> Result<RegionalTrackingQuantity, RegionalTrackingError> {
        let relative_floor = self.tracking.plan.floors[floor];
        let global = self.global(state, branch, quantity, relative_floor)?;
        let regional = self.regional(quantity.components(), clock, relative_floor)?;
        if regional.global != SampledError::Measured(global) {
            return Err(SolverError::InvalidPayload.into());
        }
        Ok(RegionalTrackingQuantity {
            quantity,
            global,
            regional,
        })
    }
    fn global(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<nsbu_solver::diagnostics::local::LocalError, RegionalTrackingError> {
        self.tracking.prepare_quantity(state, branch, quantity)?;
        Ok(self.tracking.reduce(quantity.components(), floor)?)
    }
    fn regional(
        &self,
        components: usize,
        clock: nsbu_solver::domain::TickClock,
        floor: f64,
    ) -> Result<crate::regions::RegionalReport, RegionalTrackingError> {
        match components {
            3 => self.collect::<3>(clock, floor),
            9 => self.collect::<9>(clock, floor),
            27 => self.collect::<27>(clock, floor),
            _ => Err(SolverError::InvalidPayload.into()),
        }
    }
    fn collect<const C: usize>(
        &self,
        clock: nsbu_solver::domain::TickClock,
        floor: f64,
    ) -> Result<crate::regions::RegionalReport, RegionalTrackingError> {
        let points = self.tracking.plan.samples.real_len();
        let mut errors = RegionalTensorErrors::<C>::new(
            clock,
            self.tracking.plan.samples,
            self.plan.root_budget,
            points,
            floor,
        )?;
        for (&error, &reference) in self
            .tracking
            .error_magnitudes
            .iter()
            .zip(&self.tracking.reference_magnitudes)
        {
            errors.push_magnitudes(error, reference)?;
        }
        Ok(errors.report()?)
    }
}
