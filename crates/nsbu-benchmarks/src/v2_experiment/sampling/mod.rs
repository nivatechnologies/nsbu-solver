//! Nested physical sampling of one unchanged exact-v2 trajectory family.
mod compute;
mod plan;
mod report;
use super::{physical::PhysicalFamilyWorkspace, FamilyError, V2Family};
use nsbu_solver::{domain::TickClock, SolverError};
pub use plan::{SamplingBounds, SamplingPlan, SamplingWork};
pub use report::{SamplingChange, SamplingQuantity, SamplingSample};

/// Three preallocated consumers with one schedule and bounded attempt ledger.
pub struct SamplingWorkspace<'a> {
    plan: SamplingPlan<'a>,
    physical: [PhysicalFamilyWorkspace<'a>; 3],
    charged: SamplingWork,
    next: usize,
    failed: bool,
}
impl<'a> SamplingWorkspace<'a> {
    /// Allocate only after joint admission of the family and all three consumers.
    pub fn new(plan: SamplingPlan<'a>) -> Result<Self, SolverError> {
        let [a, b, c] = plan.physical.map(PhysicalFamilyWorkspace::new);
        Ok(Self {
            plan,
            physical: [a?, b?, c?],
            charged: SamplingWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining aggregate attempts; terminal failures cannot consume retries.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Conservative work spent, including refused requests.
    pub fn charged_work(&self) -> SamplingWork {
        self.charged
    }
    /// Next required accepted V2 clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times().as_slice().get(self.next).copied()
    }
    /// Whether a child numerical failure permanently terminated this consumer.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Measure all sixty quantity/pair/lattice metrics before publishing one report.
    pub fn measure(&mut self, family: &V2Family<'_>) -> Result<SamplingSample, FamilyError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        let result = self.compute(family, clock);
        match result {
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
    fn charge(&mut self) -> Result<(), FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
}
