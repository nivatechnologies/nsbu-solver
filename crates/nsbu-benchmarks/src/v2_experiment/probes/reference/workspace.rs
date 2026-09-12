use super::{ProbeReferencePlan, ProbeReferenceSample, ProbeReferenceWork};
use crate::{
    fields::reference::ReferenceEvaluation,
    v2_experiment::{
        probes::{ProbeFamily, ProbeSample},
        reference::{self, BranchTracking, ReferenceTrackingError, TrackingQuantity, QUANTITIES},
        FamilyError,
    },
};
use nsbu_solver::{
    diagnostics::derivatives::DerivativeWorkspace, domain::TickClock, Complex64, SolverError,
};

/// Reusable analytical consumer retaining the latest complete probe report.
pub struct ProbeReferenceWorkspace<'a> {
    plan: ProbeReferencePlan<'a>,
    derivatives: [DerivativeWorkspace; 3],
    actual: Vec<f64>,
    error_magnitudes: Vec<f64>,
    reference_magnitudes: Vec<f64>,
    references: Vec<ReferenceEvaluation>,
    next: usize,
    failed: bool,
    charged: ProbeReferenceWork,
    current: Option<ProbeReferenceSample>,
}

impl<'a> ProbeReferenceWorkspace<'a> {
    /// Allocate only after complete probe-owner and consumer admission.
    pub fn new(plan: ProbeReferencePlan<'a>) -> Result<Self, SolverError> {
        let unique = [plan.sources[0], plan.sources[1], plan.sources[2]];
        let count = plan.samples.real_len();
        Ok(Self {
            derivatives: reference::tracking::derivatives(
                unique,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            actual: reference::tracking::real(count)?,
            error_magnitudes: reference::tracking::real(count)?,
            reference_magnitudes: reference::tracking::real(count)?,
            references: reference::tracking::references(count)?,
            plan,
            next: 0,
            failed: false,
            charged: ProbeReferenceWork::default(),
            current: None,
        })
    }

    /// Next exact probe-manifest clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Remaining complete attempt allowance.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Complete charged work, including a failed admitted attempt.
    pub fn charged_work(&self) -> ProbeReferenceWork {
        self.charged
    }
    /// Latest complete report, retained after later terminal failure.
    pub fn current(&self) -> Option<ProbeReferenceSample> {
        self.current
    }
    /// Track every reconstructed velocity field against one cached analytical grid.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
    ) -> Result<ProbeReferenceSample, ReferenceTrackingError> {
        if self.failed {
            return Err(ReferenceTrackingError::Terminated);
        }
        if let Err(error) = self.charge() {
            self.failed = true;
            return Err(error.into());
        }
        match self
            .bind(family, sample)
            .and_then(|clock| self.compute(family, sample, clock))
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
        self.charged.reference_evaluations = add(
            self.charged.reference_evaluations,
            one.reference_evaluations,
        )?;
        self.charged.root_iterations = add(self.charged.root_iterations, one.root_iterations)?;
        self.charged.scalar_transforms =
            add(self.charged.scalar_transforms, one.scalar_transforms)?;
        self.charged.weighted_visits = add(self.charged.weighted_visits, one.weighted_visits)?;
        self.charged.binding_checks = add(self.charged.binding_checks, one.binding_checks)?;
        Ok(())
    }

    fn bind(
        &self,
        family: &ProbeFamily<'_>,
        sample: ProbeSample,
    ) -> Result<TickClock, ReferenceTrackingError> {
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let current = family.current.ok_or(FamilyError::InvalidFamily)?;
        if family.failed
            || family.plan.identity() != self.plan.probes.identity()
            || current.identity() != self.plan.probes.identity()
            || sample.identity() != current.identity()
            || sample.clock() != clock
            || sample.clock() != current.clock()
            || sample.origins() != current.origins()
        {
            return Err(FamilyError::InvalidFamily.into());
        }
        self.bind_fields(family, sample, clock)?;
        Ok(clock)
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
        clock: TickClock,
    ) -> Result<ProbeReferenceSample, ReferenceTrackingError> {
        reference::tracking::evaluate_references(&mut self.references, self.plan.samples, clock)?;
        let branches = [
            self.branch(family, 0)?,
            self.branch(family, 1)?,
            self.branch(family, 2)?,
            self.branch(family, 3)?,
            self.branch(family, 4)?,
            self.branch(family, 5)?,
        ];
        Ok(ProbeReferenceSample {
            reconstruction,
            domains: self.plan.sources,
            samples: self.plan.samples,
            floors: self.plan.floors,
            branches,
        })
    }

    fn branch(
        &mut self,
        family: &ProbeFamily<'_>,
        branch: usize,
    ) -> Result<BranchTracking, ReferenceTrackingError> {
        let fields = family.fields(branch).ok_or(FamilyError::InvalidFamily)?;
        let quantities = [
            self.quantity(fields.value, branch, 0)?,
            self.quantity(fields.value, branch, 1)?,
            self.quantity(fields.value, branch, 2)?,
            self.quantity(fields.value, branch, 3)?,
        ];
        Ok(BranchTracking { branch, quantities })
    }

    fn quantity(
        &mut self,
        values: [&[Complex64]; 3],
        branch: usize,
        index: usize,
    ) -> Result<TrackingQuantity, ReferenceTrackingError> {
        let quantity = QUANTITIES[index];
        (reference::tracking::TrackingScratch {
            derivatives: &mut self.derivatives,
            actual: &mut self.actual,
            errors: &mut self.error_magnitudes,
            reference_magnitudes: &mut self.reference_magnitudes,
            references: &self.references,
        })
        .prepare_quantity(values, branch, quantity)?;
        let error = reference::tracking::reduce(
            quantity.components(),
            &self.error_magnitudes,
            &self.reference_magnitudes,
            self.plan.floors[index],
        )?;
        Ok(TrackingQuantity { quantity, error })
    }
}

fn add(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_add(right).ok_or(SolverError::SizeOverflow)
}
