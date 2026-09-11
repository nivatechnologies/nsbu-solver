//! Transactional six-branch doubled-band residuals at genuine exact-v2 off-stage clocks.
mod plan;
mod report;
mod workspace;
use super::{ProbeFamily, ProbeSample};
use crate::v2_experiment::{FamilyError, PAIRS};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, conservative::ConservativeWorkspace},
    domain::TickClock,
    verification::reconstruction::ProbeRefinement,
    SolverError,
};
pub use plan::{ResidualFamilyBounds, ResidualFamilyPlan, ResidualFamilyWork};
pub use report::{ResidualFamilySample, ResidualFields};
use workspace::ResidualWorkspace;
pub use workspace::{ResidualBounds, ResidualSample, ResidualWork};

/// Six fresh-force residual workspaces with one complete-report publication boundary.
pub struct ResidualFamily<'a> {
    plan: ResidualFamilyPlan<'a>,
    children: [ResidualWorkspace; 6],
    charged: ResidualFamilyWork,
    next: usize,
    failed: bool,
    current: Option<ResidualFamilySample>,
}
impl<'a> ResidualFamily<'a> {
    /// Allocate all private residual workspaces after aggregate admission.
    pub fn new(plan: ResidualFamilyPlan<'a>) -> Result<Self, SolverError> {
        let values = plan.sources.map(|source| {
            ResidualWorkspace::new(
                source,
                plan.force_samples,
                plan.bounds.work.attempts,
                plan.bounds.storage_bytes,
            )
        });
        let [a, b, c, d, e, f] = values;
        Ok(Self {
            plan,
            children: [a?, b?, c?, d?, e?, f?],
            charged: ResidualFamilyWork::default(),
            next: 0,
            failed: false,
            current: None,
        })
    }
    /// Remaining whole-report attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Complete aggregate attempted work.
    pub fn charged_work(&self) -> ResidualFamilyWork {
        self.charged
    }
    /// Actual separately retained child charges.
    pub fn child_work(&self) -> [ResidualWork; 6] {
        self.children.each_ref().map(ResidualWorkspace::consumption)
    }
    /// Next required non-stage time.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.times.get(self.next).copied()
    }
    /// Any failed binding or numerical report terminates this consumer.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Complete published residual coefficients, absent after any failed attempt.
    pub fn fields(&self, index: usize) -> Option<ResidualFields<'_>> {
        let current = self.current?;
        Some(ResidualFields {
            clock: current.clock(),
            domain: self.children.get(index)?.diagnostic_domain(),
            coefficients: self.children.get(index)?.coefficients(),
        })
    }
    /// Charge, bind and publish all six residuals and five comparisons atomically.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<ResidualFamilySample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        self.current = None;
        if self.remaining() == 0 {
            self.failed = true;
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charge();
        let result = self
            .bind(family)
            .and_then(|sample| self.compute(family, sample));
        match result {
            Ok(sample) => {
                self.next += 1;
                self.current = Some(sample);
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn bind(&self, family: &ProbeFamily<'_>) -> Result<ProbeSample, FamilyError> {
        let time = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let sample = family.current.ok_or(FamilyError::InvalidFamily)?;
        if family.failed
            || family.plan.identity != self.plan.probes.identity
            || sample.identity() != self.plan.probes.identity
            || sample.clock() != time
        {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(sample)
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        reconstruction: ProbeSample,
    ) -> Result<ResidualFamilySample, FamilyError> {
        let branches = self.collect_children(family)?;
        let comparisons = self.collect_comparisons()?;
        let geometry = plan::geometry(self.plan.probes, reconstruction.clock())?;
        for (actual, expected) in branches.iter().zip(geometry) {
            if actual.geometry().nodes() != expected.nodes() {
                return Err(FamilyError::InvalidFamily);
            }
        }
        Ok(ResidualFamilySample {
            reconstruction,
            branches,
            comparisons,
            temporal: ProbeRefinement::new([geometry[3], geometry[4], geometry[2]])?,
        })
    }
    fn collect_children(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<[ResidualSample; 6], FamilyError> {
        let values = std::array::from_fn(|index| {
            let fields = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            self.children[index]
                .measure(fields)
                .map_err(FamilyError::from)
        });
        let [a, b, c, d, e, f] = values;
        Ok([a?, b?, c?, d?, e?, f?])
    }
    fn collect_comparisons(
        &self,
    ) -> Result<[nsbu_solver::diagnostics::comparison::BandComparison; 5], FamilyError> {
        let values = PAIRS.map(|pair| self.compare(pair));
        let [a, b, c, d, e] = values;
        Ok([a?, b?, c?, d?, e?])
    }
    fn compare(
        &self,
        (a, b): (usize, usize),
    ) -> Result<nsbu_solver::diagnostics::comparison::BandComparison, SolverError> {
        ComparisonPlan::new(
            ConservativeWorkspace::diagnostic_domain(self.plan.sources[a])?,
            ConservativeWorkspace::diagnostic_domain(self.plan.sources[b])?,
        )?
        .compare(
            self.children[a].coefficients(),
            self.children[b].coefficients(),
        )
    }
    fn charge(&mut self) {
        let one = self.plan.per_attempt;
        self.charged.attempts += 1;
        self.charged.residual.probes += one.residual.probes;
        self.charged.residual.provider_work_units += one.residual.provider_work_units;
        self.charged.residual.scalar_transforms += one.residual.scalar_transforms;
        self.charged.residual.coefficient_work_units += one.residual.coefficient_work_units;
        self.charged.comparison_work_units += one.comparison_work_units;
        self.charged.binding_clock_comparisons += one.binding_clock_comparisons;
    }
}
