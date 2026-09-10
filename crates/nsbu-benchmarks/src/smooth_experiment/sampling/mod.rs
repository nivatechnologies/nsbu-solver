//! Refine physical sampling without changing, resetting or aligning any integrated state.
mod plan;
mod report;
#[cfg(test)]
mod tests;
use super::{
    physical::{PhysicalFamilyWorkspace, QuantityRefinement},
    pressure::PressureFamilyWorkspace,
    FamilyError, SmoothFamily,
};
use nsbu_solver::{domain::TickClock, SolverError};
pub use plan::{SamplingBounds, SamplingPlan, SamplingWork};
pub use report::{SamplingChange, SamplingQuantity, SamplingSample};

/// Six preallocated consumers with one complete-report schedule and bounded attempt ledger.
/// A numerical child failure terminates this consumer, preserving all trajectory histories.
pub struct SamplingWorkspace<'a> {
    plan: SamplingPlan<'a>,
    physical: [PhysicalFamilyWorkspace<'a>; 3],
    pressure: [PressureFamilyWorkspace<'a>; 3],
    charged: SamplingWork,
    next: usize,
    failed: bool,
}
impl<'a> SamplingWorkspace<'a> {
    /// Allocate only after joint admission of all owners and conservative report scratch.
    pub fn new(plan: SamplingPlan<'a>) -> Result<Self, SolverError> {
        let [a, b, c] = plan.physical.map(PhysicalFamilyWorkspace::new);
        let physical = [a?, b?, c?];
        let [a, b, c] = plan.pressure.map(PressureFamilyWorkspace::new);
        Ok(Self {
            plan,
            physical,
            pressure: [a?, b?, c?],
            charged: SamplingWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining attempts; a terminated numerical consumer cannot use these for retries.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Conservative work spent, including every refused request admitted by the ledger.
    pub fn charged_work(&self) -> SamplingWork {
        self.charged
    }
    /// Next required accepted clock; termination never silently skips a missing record.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Whether a numerical child failure has made this consumer permanently unusable.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Compute all ninety complete quantity/pair/grid measurements before publishing one report.
    /// A malformed family request spends work but can be corrected before any child advances.
    /// A child failure publishes nothing and permanently terminates the aggregate consumer.
    pub fn measure(&mut self, family: &SmoothFamily<'_>) -> Result<SamplingSample, FamilyError> {
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
        self.charged.provider_work_units += self.plan.per_attempt.provider_work_units;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &SmoothFamily<'_>,
        clock: TickClock,
    ) -> Result<SamplingSample, FamilyError> {
        let levels = [
            self.level(family, 0)?,
            self.level(family, 1)?,
            self.level(family, 2)?,
        ];
        Ok(SamplingSample {
            clock,
            layouts: self.plan.sample_layouts(),
            quantities: std::array::from_fn(|index| SamplingQuantity {
                levels: levels.map(|level| level[index]),
            }),
        })
    }
    fn level(
        &mut self,
        family: &SmoothFamily<'_>,
        index: usize,
    ) -> Result<[QuantityRefinement; 6], FamilyError> {
        let physical = self.physical[index].measure(family)?;
        let pressure = self.pressure[index].measure(family)?;
        let [a, b, c, d] = *physical.quantities();
        let [e, f] = *pressure.quantities();
        Ok([a, b, c, d, e, f])
    }
}
