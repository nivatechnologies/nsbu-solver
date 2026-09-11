//! Independently recomputed double-band balances at exact accepted-history reconstruction probes.
mod plan;
pub mod quadrature;
#[cfg(test)]
mod tests;
use super::{ProbeFamily, ProbeSample};
use crate::{
    smooth_experiment::FamilyError,
    smooth_observer::{BalanceObserver, BalanceObserverWork},
};
use nsbu_solver::{diagnostics::balances::BalanceSample, domain::TickClock, SolverError};
pub use plan::{BalanceProbeBounds, BalanceProbePlan, BalanceProbeWork};
/// Complete six-branch measured balances retaining the original accepted-node reconstruction record.
#[derive(Debug, Clone, Copy)]
pub struct BalanceProbeSample {
    origin: ProbeSample,
    branches: [BalanceSample; 6],
}
impl BalanceProbeSample {
    /// Physical probe time, distinct from accepted lookahead endpoints.
    pub fn clock(self) -> TickClock {
        self.origin.clock()
    }
    /// Exact original node origins and full velocity/derivative probe comparisons.
    pub fn reconstruction(&self) -> &ProbeSample {
        &self.origin
    }
    /// All six conservative balance samples, in the original family branch order.
    pub fn branches(&self) -> &[BalanceSample; 6] {
        &self.branches
    }
}
/// A complete-report boundary around six independently owned conservative product/force observers.
/// This diagnostic consumer cannot mutate trajectories or their reconstructed field scratch.
pub struct BalanceProbes<'a> {
    plan: BalanceProbePlan<'a>,
    children: [BalanceObserver; 6],
    charged: BalanceProbeWork,
    next: usize,
    failed: bool,
}
impl<'a> BalanceProbes<'a> {
    /// Allocate independent observers only after full simultaneous resource admission.
    pub fn new(plan: BalanceProbePlan<'a>) -> Result<Self, SolverError> {
        let [a, b, c, d, e, f] = plan
            .resources
            .map(|resource| BalanceObserver::new(resource, plan.bounds.work.attempts));
        Ok(Self {
            plan,
            children: [a?, b?, c?, d?, e?, f?],
            charged: BalanceProbeWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining finite aggregate attempts, including refusals before child evaluation.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Conservative complete-report charges, including failed requests.
    pub fn charged_work(&self) -> BalanceProbeWork {
        self.charged
    }
    /// Actual child sample/provider/transform ledgers, retained separately from aggregate allowances.
    pub fn child_work(&self) -> [BalanceObserverWork; 6] {
        self.children.each_ref().map(BalanceObserver::consumption)
    }
    /// Next required exact probe time; earlier missing measurements cannot be skipped.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// A prior child failure prohibits further attempts or partial-report recovery.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Invalid input consumes an aggregate attempt. A child failure is terminal and publishes nothing.
    pub fn measure(&mut self, family: &ProbeFamily<'_>) -> Result<BalanceProbeSample, FamilyError> {
        self.charge()?;
        let origin = self.plan.probes.require_sample(family, self.next)?;
        match self.compute(family, origin) {
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
        let one = self.plan.one;
        self.charged.attempts += 1;
        self.charged.children.samples += one.children.samples;
        self.charged.children.work_units += one.children.work_units;
        self.charged.children.scalar_transforms += one.children.scalar_transforms;
        self.charged.binding_comparisons += one.binding_comparisons;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        origin: ProbeSample,
    ) -> Result<BalanceProbeSample, FamilyError> {
        let mut measure = |index| -> Result<BalanceSample, FamilyError> {
            let fields = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            // require_sample and fields borrow the same private current probe record.
            // Its original clock and node origins cannot change during this measurement.
            Ok(self.children[index].sample_probe(fields.domain, fields.clock, fields.value)?)
        };
        Ok(BalanceProbeSample {
            origin,
            branches: [
                measure(0)?,
                measure(1)?,
                measure(2)?,
                measure(3)?,
                measure(4)?,
                measure(5)?,
            ],
        })
    }
}
