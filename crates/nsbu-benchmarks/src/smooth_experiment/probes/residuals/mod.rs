//! Stream independent conservative PDE residuals before early accepted histories are overwritten.
mod plan;
mod report;
#[cfg(test)]
mod tests;
use super::{ProbeFamily, ProbeSample};
use crate::smooth_experiment::{
    residual::{ResidualSample, ResidualWork, ResidualWorkspace},
    FamilyError, PAIRS,
};
use nsbu_solver::{
    diagnostics::{
        comparison::{BandComparison, ComparisonPlan},
        conservative::ConservativeWorkspace,
    },
    domain::TickClock,
    verification::reconstruction::ProbeRefinement,
    SolverError,
};
pub use plan::{ResidualFamilyBounds, ResidualFamilyPlan, ResidualFamilyWork};
pub use report::ResidualFamilySample;

/// Six independently reconstructed diagnostic paths with a complete-report transaction boundary.
/// Trajectories and their interpolants stay borrowed; no reference or mutable state is accepted.
pub struct ResidualFamily<'a> {
    plan: ResidualFamilyPlan<'a>,
    children: [ResidualWorkspace; 6],
    charged: ResidualFamilyWork,
    next: usize,
    failed: bool,
}
impl<'a> ResidualFamily<'a> {
    /// Construct all separately budgeted residual workspaces only after full joint admission.
    pub fn new(plan: ResidualFamilyPlan<'a>) -> Result<Self, SolverError> {
        let [a, b, c, d, e, f] = plan.sources.map(|source| {
            ResidualWorkspace::new(source, plan.bounds.work.attempts, plan.bounds.storage_bytes)
        });
        Ok(Self {
            plan,
            children: [a?, b?, c?, d?, e?, f?],
            charged: ResidualFamilyWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining finite aggregate attempts; termination prohibits any further retry.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Complete conservative charges, including requests rejected before child evaluation.
    pub fn charged_work(&self) -> ResidualFamilyWork {
        self.charged
    }
    /// Work actually charged by each child; retained separately from aggregate worst-case allowances.
    pub fn child_work(&self) -> [ResidualWork; 6] {
        self.children.each_ref().map(ResidualWorkspace::consumption)
    }
    /// Next required non-stage time; no missing earlier residual can be skipped.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.times.get(self.next).copied()
    }
    /// A numerical child or aggregate comparison failure permanently terminates this consumer.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Bind the current complete probe before assembling six defects and five full-field comparisons.
    /// Malformed requests spend work without advancing children; numerical failure publishes nothing.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<ResidualFamilySample, FamilyError> {
        self.charge()?;
        let time = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let index = self
            .plan
            .probes
            .tested_times()
            .as_slice()
            .iter()
            .position(|&t| t == time)
            .ok_or(FamilyError::InvalidFamily)?;
        let origin = self.plan.probes.require_sample(family, index)?;
        match self.compute(family, origin) {
            Ok(report) => {
                self.next += 1;
                Ok(report)
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
        let one = self.plan.per_attempt;
        self.charged.attempts += 1;
        self.charged.residual.probes += one.residual.probes;
        self.charged.residual.provider_work_units += one.residual.provider_work_units;
        self.charged.residual.scalar_transforms += one.residual.scalar_transforms;
        self.charged.residual.coefficient_work_units += one.residual.coefficient_work_units;
        self.charged.comparison_work_units += one.comparison_work_units;
        self.charged.binding_clock_comparisons += one.binding_clock_comparisons;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        origin: ProbeSample,
    ) -> Result<ResidualFamilySample, FamilyError> {
        let mut measure = |index| self.branch(family, origin, index);
        let branches = [
            measure(0)?,
            measure(1)?,
            measure(2)?,
            measure(3)?,
            measure(4)?,
            measure(5)?,
        ];
        let compare = |pair| self.compare(pair);
        let [a, b, c, d, e] = PAIRS.map(compare);
        let temporal = ProbeRefinement::new([
            branches[3].geometry(),
            branches[4].geometry(),
            branches[2].geometry(),
        ])?;
        Ok(ResidualFamilySample {
            origin,
            branches,
            comparisons: [a?, b?, c?, d?, e?],
            temporal,
        })
    }
    fn branch(
        &mut self,
        family: &ProbeFamily<'_>,
        origin: ProbeSample,
        index: usize,
    ) -> Result<ResidualSample, FamilyError> {
        let run = family.branch(index).ok_or(FamilyError::InvalidFamily)?;
        let result = self.children[index].measure(run, origin.clock())?;
        if result.geometry().nodes() != origin.origins()[index].accepted_nodes {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(result)
    }
    fn compare(&self, (a, b): (usize, usize)) -> Result<BandComparison, SolverError> {
        ComparisonPlan::new(
            ConservativeWorkspace::diagnostic_domain(self.plan.sources[a])?,
            ConservativeWorkspace::diagnostic_domain(self.plan.sources[b])?,
        )?
        .compare(
            self.children[a].coefficients(),
            self.children[b].coefficients(),
        )
    }
}
