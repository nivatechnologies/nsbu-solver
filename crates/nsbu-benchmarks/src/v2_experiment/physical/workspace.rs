//! Reusable physical comparison workspace and bounded schedule ledger.
mod compute;
use super::{PhysicalFamilyPlan, PhysicalFamilyWork, PhysicalRefinementSample};
use crate::v2_experiment::{FamilyError, V2Family};
use nsbu_solver::{
    diagnostics::physical::PhysicalComparisonWorkspace, domain::TickClock, SolverError,
};

/// Reusable bounded physical comparison workspace for one V2 family.
pub struct PhysicalFamilyWorkspace<'a> {
    pub(super) plan: PhysicalFamilyPlan<'a>,
    pub(super) workspace: PhysicalComparisonWorkspace,
    charged: PhysicalFamilyWork,
    next: usize,
}
impl<'a> PhysicalFamilyWorkspace<'a> {
    /// Construct from an already jointly admitted plan.
    pub fn new(plan: PhysicalFamilyPlan<'a>) -> Result<Self, SolverError> {
        let domain = plan.family.branches[2].resources().domain();
        Ok(Self {
            workspace: PhysicalComparisonWorkspace::new(
                domain,
                domain,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            plan,
            charged: PhysicalFamilyWork::default(),
            next: 0,
        })
    }
    /// Remaining finite measurement attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Charged work, including failed calls.
    pub fn charged_work(&self) -> PhysicalFamilyWork {
        self.charged
    }
    /// Next required family clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Compare all quantities at the next accepted family clock.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<PhysicalRefinementSample, FamilyError> {
        self.charge()?;
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        self.plan.family.require_sample(family, self.next)?;
        let quantities = self.quantities(family)?;
        self.next += 1;
        Ok(PhysicalRefinementSample {
            clock,
            identity: self.plan.family.identity(),
            samples: self.plan.samples,
            floors: self.plan.floors,
            quantities,
        })
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.attempts += 1;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
}
