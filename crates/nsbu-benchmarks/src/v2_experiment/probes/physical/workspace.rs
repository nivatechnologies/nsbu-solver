//! Constant-storage transaction over one complete current probe publication.
use super::{
    ProbePhysicalPlan, ProbePhysicalQuantity, ProbePhysicalSample, ProbePhysicalWork,
    PROBE_PHYSICAL_QUANTITIES,
};
use crate::v2_experiment::{
    physical::PhysicalExtrema,
    probes::{ProbeFamily, ProbeSample},
    FamilyError, PAIRS,
};
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    domain::TickClock,
    SolverError,
};

/// Reusable physical consumer with one retained complete report.
pub struct ProbePhysicalWorkspace<'a> {
    plan: ProbePhysicalPlan<'a>,
    workspace: PhysicalComparisonWorkspace,
    next: usize,
    failed: bool,
    charged: ProbePhysicalWork,
    current: Option<ProbePhysicalSample>,
}
impl<'a> ProbePhysicalWorkspace<'a> {
    /// Allocate only after joint probe-owner and consumer admission.
    pub fn new(plan: ProbePhysicalPlan<'a>) -> Result<Self, SolverError> {
        let finest = plan.sources[2];
        Ok(Self {
            workspace: PhysicalComparisonWorkspace::new(
                finest,
                finest,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            plan,
            next: 0,
            failed: false,
            charged: ProbePhysicalWork::default(),
            current: None,
        })
    }
    /// Next exact manifest clock required from the probe producer.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Remaining whole-attempt allowance.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Complete work charged, including a failed attempt.
    pub fn charged_work(&self) -> ProbePhysicalWork {
        self.charged
    }
    /// Most recent complete report, retained after a later terminal failure.
    pub fn current(&self) -> Option<ProbePhysicalSample> {
        self.current
    }
    /// Measure every physical quantity and pair from reconstructed velocity only.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
    ) -> Result<ProbePhysicalSample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        if let Err(error) = self.charge() {
            self.failed = true;
            return Err(error.into());
        }
        match self
            .bind(family, sample)
            .and_then(|_| self.compute(family, sample))
        {
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
        let one = self.plan.per_attempt;
        self.charged.attempts += 1;
        self.charged.scalar_transforms = self
            .charged
            .scalar_transforms
            .checked_add(one.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.weighted_visits = self
            .charged
            .weighted_visits
            .checked_add(one.weighted_visits)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.binding_checks = self
            .charged
            .binding_checks
            .checked_add(one.binding_checks)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }
    fn bind(&self, family: &ProbeFamily<'_>, sample: ProbeSample) -> Result<(), FamilyError> {
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let current = family.current.ok_or(FamilyError::InvalidFamily)?;
        self.bind_publication(family, sample, current, clock)?;
        self.bind_fields(family, sample, clock)
    }
    fn bind_publication(
        &self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
        current: ProbeSample,
        clock: TickClock,
    ) -> Result<(), FamilyError> {
        if family.failed
            || family.plan.identity() != self.plan.probes.identity()
            || current.identity() != self.plan.probes.identity()
            || sample.identity() != current.identity()
            || sample.clock() != clock
            || sample.clock() != current.clock()
            || sample.origins() != current.origins()
        {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(())
    }
    fn bind_fields(
        &self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
        clock: TickClock,
    ) -> Result<(), FamilyError> {
        for index in 0..6 {
            let fields = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            if fields.clock != clock
                || fields.domain != self.plan.sources[index]
                || fields.origin != sample.origins()[index]
            {
                return Err(FamilyError::InvalidFamily);
            }
        }
        Ok(())
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        reconstruction: ProbeSample,
    ) -> Result<ProbePhysicalSample, FamilyError> {
        let quantities = [
            self.quantity(family, 0)?,
            self.quantity(family, 1)?,
            self.quantity(family, 2)?,
            self.quantity(family, 3)?,
        ];
        Ok(ProbePhysicalSample {
            reconstruction,
            domains: self.plan.sources,
            samples: self.plan.samples,
            floors: self.plan.floors,
            quantities,
        })
    }
    fn quantity(
        &mut self,
        family: &ProbeFamily<'_>,
        index: usize,
    ) -> Result<ProbePhysicalQuantity, FamilyError> {
        let quantity = PROBE_PHYSICAL_QUANTITIES[index];
        let floor = self.plan.floors[index];
        let values = PAIRS.map(|pair| self.pair(family, pair, quantity, floor));
        let [a, b, c, d, e] = values;
        let [a, b, c, d, e] = [a?, b?, c?, d?, e?];
        Ok(ProbePhysicalQuantity::new(
            quantity,
            [a.0, b.0, c.0, d.0, e.0],
            [a.1, b.1, c.1, d.1, e.1],
        ))
    }
    fn pair(
        &mut self,
        family: &ProbeFamily<'_>,
        (left, right): (usize, usize),
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<(nsbu_solver::diagnostics::local::LocalError, PhysicalExtrema), FamilyError> {
        let left = family.fields(left).ok_or(FamilyError::InvalidFamily)?;
        let right = family.fields(right).ok_or(FamilyError::InvalidFamily)?;
        let comparison = self.workspace.compare_domains(
            [left.domain, right.domain],
            PhysicalField::Vector(left.value),
            PhysicalField::Vector(right.value),
            quantity,
            floor,
        )?;
        Ok((
            comparison.global(),
            PhysicalExtrema::from_comparison(&comparison, floor)?,
        ))
    }
}
