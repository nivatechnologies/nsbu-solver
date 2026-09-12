//! Transactional pressure comparisons over one complete current probe publication.
use super::{ProbePressurePlan, ProbePressureQuantity, ProbePressureSample, ProbePressureWork};
use crate::{
    runtime_force::RunForce,
    v2_experiment::{
        pressure::{construct_pressure, PressureScratch},
        probes::{ProbeFamily, ProbeSample},
        FamilyError, PAIRS,
    },
};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace,
        local::LocalError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::TickClock,
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
type Field = [Vec<Complex64>; 3];
/// Reusable bounded off-stage pressure consumer.
pub struct ProbePressureWorkspace<'a> {
    plan: ProbePressurePlan<'a>,
    products: ConservativeWorkspace,
    comparison: PhysicalComparisonWorkspace,
    provider: RunForce,
    velocity: Field,
    force: Field,
    conservative: Field,
    pressure: [Vec<Complex64>; 2],
    next: usize,
    failed: bool,
    charged: ProbePressureWork,
    current: Option<ProbePressureSample>,
}
impl<'a> ProbePressureWorkspace<'a> {
    /// Allocate only after complete joint probe-owner and consumer admission.
    pub fn new(plan: ProbePressurePlan<'a>) -> Result<Self, SolverError> {
        let n = plan.source.layout().half_len();
        let m = plan.diagnostic.layout().half_len();
        Ok(Self {
            products: ConservativeWorkspace::new(plan.source, plan.bounds.storage_bytes)?,
            comparison: PhysicalComparisonWorkspace::new(
                plan.diagnostic,
                plan.diagnostic,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            provider: plan
                .force
                .build(plan.diagnostic, plan.provider_limits.storage_bytes)?,
            velocity: field(n)?,
            force: field(m)?,
            conservative: field(m)?,
            pressure: [filled(m)?, filled(m)?],
            next: 0,
            failed: false,
            charged: ProbePressureWork::default(),
            current: None,
            plan,
        })
    }
    /// Next exact probe-manifest clock required for publication.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Remaining whole-report attempts, including possible failed attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Work charged before each attempt; failed attempts remain visible.
    pub fn charged_work(&self) -> ProbePressureWork {
        self.charged
    }
    /// Most recent complete report, retained after later terminal failure.
    pub fn current(&self) -> Option<ProbePressureSample> {
        self.current
    }
    /// Bind all six current reconstructed value fields, then construct and compare pressure transactionally.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
    ) -> Result<ProbePressureSample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        };
        if let Err(e) = self.charge() {
            self.failed = true;
            return Err(e.into());
        }
        let result = self
            .bind(family, sample)
            .and_then(|clock| self.compute(family, sample, clock));
        match result {
            Ok(report) => {
                self.next += 1;
                self.current = Some(report);
                Ok(report)
            }
            Err(e) => {
                self.failed = true;
                Err(e)
            }
        }
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        };
        let w = self.plan.per_attempt;
        self.charged.attempts += 1;
        self.charged.provider_work_units = self
            .charged
            .provider_work_units
            .checked_add(w.provider_work_units)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.scalar_transforms = self
            .charged
            .scalar_transforms
            .checked_add(w.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.weighted_visits = self
            .charged
            .weighted_visits
            .checked_add(w.weighted_visits)
            .ok_or(SolverError::SizeOverflow)?;
        self.charged.binding_checks = self
            .charged
            .binding_checks
            .checked_add(w.binding_checks)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }
    fn bind(
        &self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
    ) -> Result<TickClock, FamilyError> {
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let current = family.current.ok_or(FamilyError::InvalidFamily)?;
        self.bind_publication(family, sample, current, clock)?;
        self.bind_fields(family, sample, clock)?;
        Ok(clock)
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
            let f = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            if f.clock != clock
                || f.domain != self.plan.sources[index]
                || f.origin != sample.origins()[index]
            {
                return Err(FamilyError::InvalidFamily);
            }
        }
        Ok(())
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
        clock: TickClock,
    ) -> Result<ProbePressureSample, FamilyError> {
        self.evaluate_force(clock)?;
        let values = PAIRS.map(|pair| self.pair(family, pair));
        let [a, b, c, d, e] = values;
        let pairs = [a?, b?, c?, d?, e?];
        Ok(ProbePressureSample {
            reconstruction: sample,
            source: self.plan.source,
            diagnostic: self.plan.diagnostic,
            samples: self.plan.samples,
            force_workers: self.plan.force.workers,
            floors: self.plan.floors,
            quantities: [
                ProbePressureQuantity {
                    quantity: PhysicalQuantity::Scalar,
                    pairs: pairs.map(|p| p[0]),
                },
                ProbePressureQuantity {
                    quantity: PhysicalQuantity::ScalarGradient,
                    pairs: pairs.map(|p| p[1]),
                },
            ],
        })
    }
    fn evaluate_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        let work = self.provider.evaluate(
            clock,
            self.plan.provider_limits,
            self.force.each_mut().map(Vec::as_mut_slice),
        )?;
        if work.work_units > self.plan.provider_limits.work_units
            || work.scalar_transforms > self.plan.provider_limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn pair(
        &mut self,
        family: &ProbeFamily<'_>,
        (a, b): (usize, usize),
    ) -> Result<[LocalError; 2], FamilyError> {
        self.construct(family, a, 0)?;
        self.construct(family, b, 1)?;
        Ok([
            self.compare(PhysicalQuantity::Scalar, 0)?,
            self.compare(PhysicalQuantity::ScalarGradient, 1)?,
        ])
    }
    fn construct(
        &mut self,
        family: &ProbeFamily<'_>,
        branch: usize,
        side: usize,
    ) -> Result<(), FamilyError> {
        let f = family.fields(branch).ok_or(FamilyError::InvalidFamily)?;
        construct_pressure(
            PressureScratch {
                products: &mut self.products,
                velocity: &mut self.velocity,
                force: &self.force,
                conservative: &mut self.conservative,
            },
            self.plan.source,
            f.domain,
            f.value,
            &mut self.pressure[side],
        )?;
        Ok(())
    }
    fn compare(&mut self, q: PhysicalQuantity, i: usize) -> Result<LocalError, SolverError> {
        Ok(self
            .comparison
            .compare(
                PhysicalField::Scalar(&self.pressure[0]),
                PhysicalField::Scalar(&self.pressure[1]),
                q,
                self.plan.floors[i],
            )?
            .global())
    }
}
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    v.resize(n, Complex64::new(0.0, 0.0));
    Ok(v)
}
