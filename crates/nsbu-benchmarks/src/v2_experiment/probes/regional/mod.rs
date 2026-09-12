//! Five-class regional analytical errors for actual reconstructed probe fields.
mod plan;
mod report;

use crate::{
    fields::reference::ReferenceEvaluation,
    regions::{RegionalError, RegionalTensorErrors},
    v2_experiment::{
        probes::{reference::ProbeReferenceSample, ProbeFamily, ProbeSample},
        reference::{
            self,
            regional::{RegionalTrackingQuantity, RegionalTrackingWork},
            ReferenceTrackingError, QUANTITIES,
        },
        FamilyError,
    },
};
use nsbu_solver::{
    diagnostics::{derivatives::DerivativeWorkspace, local::SampledError},
    domain::TickClock,
    Complex64, SolverError,
};
pub use plan::{ProbeRegionalBounds, ProbeRegionalPlan};
pub use report::{ProbeRegionalBranch, ProbeRegionalSample, ProbeRegionalStatus};

/// Regional failures preserve whether analytical reconstruction or geometry failed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbeRegionalError {
    /// Probe publication, analytical reference, transform or reduction failure.
    Tracking(ReferenceTrackingError),
    /// Regional classification or measurement failure.
    Regional(RegionalError),
}
impl From<ReferenceTrackingError> for ProbeRegionalError {
    fn from(value: ReferenceTrackingError) -> Self {
        Self::Tracking(value)
    }
}
impl From<FamilyError> for ProbeRegionalError {
    fn from(value: FamilyError) -> Self {
        Self::Tracking(value.into())
    }
}
impl From<SolverError> for ProbeRegionalError {
    fn from(value: SolverError) -> Self {
        Self::Tracking(value.into())
    }
}
impl From<RegionalError> for ProbeRegionalError {
    fn from(value: RegionalError) -> Self {
        Self::Regional(value)
    }
}

/// Independent scratch retaining only the latest complete regional report.
pub struct ProbeRegionalWorkspace<'a> {
    plan: ProbeRegionalPlan<'a>,
    derivatives: [DerivativeWorkspace; 3],
    actual: Vec<f64>,
    errors: Vec<f64>,
    reference_magnitudes: Vec<f64>,
    references: Vec<ReferenceEvaluation>,
    next: usize,
    failed: bool,
    charged_tracking: crate::v2_experiment::probes::reference::ProbeReferenceWork,
    charged_regional: RegionalTrackingWork,
    current: Option<ProbeRegionalSample>,
}

impl<'a> ProbeRegionalWorkspace<'a> {
    /// Allocate the independent analytical and classification scratch after admission.
    pub fn new(plan: ProbeRegionalPlan<'a>) -> Result<Self, SolverError> {
        let unique = [plan.sources[0], plan.sources[1], plan.sources[2]];
        let count = plan.reference.sample_layout().real_len();
        Ok(Self {
            derivatives: reference::tracking::derivatives(
                unique,
                plan.reference.sample_layout(),
                plan.bounds.storage_bytes,
            )?,
            actual: reference::tracking::real(count)?,
            errors: reference::tracking::real(count)?,
            reference_magnitudes: reference::tracking::real(count)?,
            references: reference::tracking::references(count)?,
            plan,
            next: 0,
            failed: false,
            charged_tracking: Default::default(),
            charged_regional: Default::default(),
            current: None,
        })
    }
    /// Next exact clock in the admitted seven-clock probe manifest.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .reference
            .probe_plan()
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Remaining whole attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged_tracking.attempts
    }
    /// Charged independent analytical work, including a failed attempt.
    pub fn tracking_work(&self) -> crate::v2_experiment::probes::reference::ProbeReferenceWork {
        self.charged_tracking
    }
    /// Charged regional classification work, including a failed attempt.
    pub fn regional_work(&self) -> RegionalTrackingWork {
        self.charged_regional
    }
    /// Most recent complete report, retained after a later failure.
    pub fn current(&self) -> Option<ProbeRegionalSample> {
        self.current
    }

    /// Recompute globals, require exact agreement with the bound reference report, then classify.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
        probe: ProbeSample,
        reference_sample: ProbeReferenceSample,
    ) -> Result<ProbeRegionalSample, ProbeRegionalError> {
        if self.failed {
            return Err(ReferenceTrackingError::Terminated.into());
        }
        self.charge()?;
        let result = self
            .bind(family, probe, reference_sample)
            .and_then(|clock| self.compute(family, reference_sample, clock));
        match result {
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

    fn charge(&mut self) -> Result<(), ProbeRegionalError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        add_tracking(&mut self.charged_tracking, self.plan.per_tracking)?;
        add_regional(&mut self.charged_regional, self.plan.per_regional)?;
        Ok(())
    }

    fn bind(
        &self,
        family: &ProbeFamily<'_>,
        probe: ProbeSample,
        supplied: ProbeReferenceSample,
    ) -> Result<TickClock, ProbeRegionalError> {
        let expected = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let current = family.current.ok_or(FamilyError::InvalidFamily)?;
        if family.failed
            || family.plan.identity() != self.plan.reference.probe_plan().identity()
            || current.identity() != probe.identity()
            || probe.identity() != supplied.identity()
            || probe.clock() != expected
            || supplied.clock() != expected
            || probe.origins() != supplied.origins()
            || probe.values() != supplied.reconstruction().values()
            || probe.derivatives() != supplied.reconstruction().derivatives()
            || supplied.sample_layout() != self.plan.reference.sample_layout()
            || supplied.relative_floors() != self.plan.reference.relative_floors()
            || supplied.source_domains() != self.plan.sources
        {
            return Err(FamilyError::InvalidFamily.into());
        }
        for index in 0..6 {
            let fields = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            if fields.clock != expected
                || fields.domain != self.plan.sources[index]
                || fields.origin != probe.origins()[index]
            {
                return Err(FamilyError::InvalidFamily.into());
            }
        }
        Ok(expected)
    }

    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        supplied: ProbeReferenceSample,
        clock: TickClock,
    ) -> Result<ProbeRegionalSample, ProbeRegionalError> {
        reference::tracking::evaluate_references(
            &mut self.references,
            self.plan.reference.sample_layout(),
            clock,
        )?;
        let branches = [
            self.branch(family, supplied, 0, clock)?,
            self.branch(family, supplied, 1, clock)?,
            self.branch(family, supplied, 2, clock)?,
            self.branch(family, supplied, 3, clock)?,
            self.branch(family, supplied, 4, clock)?,
            self.branch(family, supplied, 5, clock)?,
        ];
        Ok(ProbeRegionalSample {
            reference: supplied,
            branches,
        })
    }

    fn branch(
        &mut self,
        family: &ProbeFamily<'_>,
        supplied: ProbeReferenceSample,
        branch: usize,
        clock: TickClock,
    ) -> Result<ProbeRegionalBranch, ProbeRegionalError> {
        let values = family
            .fields(branch)
            .ok_or(FamilyError::InvalidFamily)?
            .value;
        Ok(ProbeRegionalBranch {
            branch,
            quantities: [
                self.quantity(values, supplied, branch, 0, clock)?,
                self.quantity(values, supplied, branch, 1, clock)?,
                self.quantity(values, supplied, branch, 2, clock)?,
                self.quantity(values, supplied, branch, 3, clock)?,
            ],
        })
    }

    fn quantity(
        &mut self,
        values: [&[Complex64]; 3],
        supplied: ProbeReferenceSample,
        branch: usize,
        index: usize,
        clock: TickClock,
    ) -> Result<RegionalTrackingQuantity, ProbeRegionalError> {
        let quantity = QUANTITIES[index];
        (reference::tracking::TrackingScratch {
            derivatives: &mut self.derivatives,
            actual: &mut self.actual,
            errors: &mut self.errors,
            reference_magnitudes: &mut self.reference_magnitudes,
            references: &self.references,
        })
        .prepare_quantity(values, branch, quantity)?;
        let global = reference::tracking::reduce(
            quantity.components(),
            &self.errors,
            &self.reference_magnitudes,
            self.plan.reference.relative_floors()[index],
        )?;
        if global != supplied.branches()[branch].quantities[index].error {
            return Err(SolverError::InvalidPayload.into());
        }
        let regional = self.collect(
            quantity.components(),
            clock,
            self.plan.reference.relative_floors()[index],
        )?;
        if regional.global != SampledError::Measured(global) {
            return Err(SolverError::InvalidPayload.into());
        }
        Ok(RegionalTrackingQuantity {
            quantity,
            global,
            regional,
        })
    }

    fn collect(
        &self,
        components: usize,
        clock: TickClock,
        floor: f64,
    ) -> Result<crate::regions::RegionalReport, ProbeRegionalError> {
        match components {
            3 => self.collect_typed::<3>(clock, floor),
            9 => self.collect_typed::<9>(clock, floor),
            27 => self.collect_typed::<27>(clock, floor),
            _ => Err(SolverError::InvalidPayload.into()),
        }
    }
    fn collect_typed<const C: usize>(
        &self,
        clock: TickClock,
        floor: f64,
    ) -> Result<crate::regions::RegionalReport, ProbeRegionalError> {
        let points = self.plan.reference.sample_layout().real_len();
        let mut collector = RegionalTensorErrors::<C>::new(
            clock,
            self.plan.reference.sample_layout(),
            self.plan.root_budget,
            points,
            floor,
        )?;
        for (&error, &reference) in self.errors.iter().zip(&self.reference_magnitudes) {
            collector.push_magnitudes(error, reference)?;
        }
        Ok(collector.report()?)
    }
}

fn add_tracking(
    a: &mut crate::v2_experiment::probes::reference::ProbeReferenceWork,
    b: crate::v2_experiment::probes::reference::ProbeReferenceWork,
) -> Result<(), SolverError> {
    a.attempts = add(a.attempts, b.attempts)?;
    a.reference_evaluations = add(a.reference_evaluations, b.reference_evaluations)?;
    a.root_iterations = add(a.root_iterations, b.root_iterations)?;
    a.scalar_transforms = add(a.scalar_transforms, b.scalar_transforms)?;
    a.weighted_visits = add(a.weighted_visits, b.weighted_visits)?;
    a.binding_checks = add(a.binding_checks, b.binding_checks)?;
    Ok(())
}
fn add_regional(a: &mut RegionalTrackingWork, b: RegionalTrackingWork) -> Result<(), SolverError> {
    a.attempts = add(a.attempts, b.attempts)?;
    a.classifications = add(a.classifications, b.classifications)?;
    a.root_iterations = add(a.root_iterations, b.root_iterations)?;
    a.magnitude_visits = add(a.magnitude_visits, b.magnitude_visits)?;
    Ok(())
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
