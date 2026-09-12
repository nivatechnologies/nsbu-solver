//! Constant-storage transactional measurement over borrowed force-family states.
use super::{
    ForcePhysicalError, ForcePhysicalPlan, ForcePhysicalQuantity, ForcePhysicalSample,
    ForcePhysicalWork, FORCE_PHYSICAL_QUANTITIES,
};
use crate::{
    v2_experiment::physical::PhysicalExtrema,
    v2_force_experiment::{ForceFamily, ForceRefinementSample},
};
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    domain::{SpectralState, TickClock},
    SolverError,
};

/// Reusable consumer with one retained complete report and no per-attempt allocation.
pub struct ForcePhysicalWorkspace<'a> {
    plan: ForcePhysicalPlan<'a>,
    workspace: PhysicalComparisonWorkspace,
    next: usize,
    failed: bool,
    charged: ForcePhysicalWork,
    current: Option<ForcePhysicalSample>,
}
impl<'a> ForcePhysicalWorkspace<'a> {
    /// Allocate only after family plus consumer storage has been jointly admitted.
    pub fn new(plan: ForcePhysicalPlan<'a>) -> Result<Self, ForcePhysicalError> {
        let domain = plan
            .family
            .branch_plan(0)
            .ok_or(ForcePhysicalError::InvalidFamily)?
            .resources()
            .domain();
        Ok(Self {
            workspace: PhysicalComparisonWorkspace::new(
                domain,
                domain,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            plan,
            next: 0,
            failed: false,
            charged: ForcePhysicalWork::default(),
            current: None,
        })
    }
    /// Next exact family clock required by the consumer.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times().as_slice().get(self.next).copied()
    }
    /// Remaining whole-attempt allowance.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Complete charged ledger, including a terminal failed attempt.
    pub fn charged_work(&self) -> ForcePhysicalWork {
        self.charged
    }
    /// Most recent complete report, retained if a later request fails.
    pub fn current(&self) -> Option<ForcePhysicalSample> {
        self.current
    }
    /// Measure two force-grid pairs from one synchronized actual family publication.
    pub fn measure(
        &mut self,
        family: &ForceFamily<'_>,
        raw: ForceRefinementSample,
    ) -> Result<ForcePhysicalSample, ForcePhysicalError> {
        if self.failed {
            return Err(ForcePhysicalError::Terminated);
        }
        if let Err(error) = self.charge() {
            self.failed = true;
            return Err(error.into());
        }
        match self.compute(family, raw) {
            Ok(report) => {
                self.next += 1;
                self.current = Some(report);
                Ok(report)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.attempts += 1;
        self.charged.scalar_transforms = self
            .charged
            .scalar_transforms
            .checked_add(self.plan.per_attempt.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.weighted_visits = self
            .charged
            .weighted_visits
            .checked_add(self.plan.per_attempt.weighted_visits)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &ForceFamily<'_>,
        raw: ForceRefinementSample,
    ) -> Result<ForcePhysicalSample, ForcePhysicalError> {
        let clock = self
            .plan
            .family
            .require_sample(family, self.next)
            .map_err(|_| ForcePhysicalError::InvalidFamily)?;
        if raw.clock() != clock || raw.identity() != self.plan.family.identity() {
            return Err(ForcePhysicalError::InvalidFamily);
        }
        let quantities = [
            self.quantity(family, 0)?,
            self.quantity(family, 1)?,
            self.quantity(family, 2)?,
            self.quantity(family, 3)?,
        ];
        Ok(ForcePhysicalSample {
            clock,
            identity: self.plan.family.identity(),
            settings: self.plan.family.settings(),
            samples: self.plan.samples,
            floors: self.plan.floors,
            quantities,
        })
    }
    fn quantity(
        &mut self,
        family: &ForceFamily<'_>,
        index: usize,
    ) -> Result<ForcePhysicalQuantity, ForcePhysicalError> {
        let quantity = FORCE_PHYSICAL_QUANTITIES[index];
        let floor = self.plan.floors[index];
        let a = self.pair(family, 0, 1, quantity, floor)?;
        let b = self.pair(family, 1, 2, quantity, floor)?;
        Ok(ForcePhysicalQuantity::new(quantity, [a.0, b.0], [a.1, b.1]))
    }
    fn pair(
        &mut self,
        family: &ForceFamily<'_>,
        left: usize,
        right: usize,
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<(nsbu_solver::diagnostics::local::LocalError, PhysicalExtrema), ForcePhysicalError>
    {
        let left = family
            .branch(left)
            .ok_or(ForcePhysicalError::InvalidFamily)?
            .state();
        let right = family
            .branch(right)
            .ok_or(ForcePhysicalError::InvalidFamily)?
            .state();
        let comparison = self
            .workspace
            .compare(field(left)?, field(right)?, quantity, floor)?;
        Ok((
            comparison.global(),
            PhysicalExtrema::from_comparison(&comparison, floor)?,
        ))
    }
}

fn field(state: &SpectralState) -> Result<PhysicalField<'_>, SolverError> {
    Ok(PhysicalField::Vector([
        state.component(0)?,
        state.component(1)?,
        state.component(2)?,
    ]))
}
