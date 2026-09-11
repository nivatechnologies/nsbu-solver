//! Transactional exact-v2 balances at reconstructed physical probe clocks.
mod plan;
pub mod quadrature;
#[cfg(test)]
mod tests;
use super::{ProbeFamily, ProbeSample};
use crate::{
    smooth_observer::{v2::V2Observer, BalanceObserverWork},
    v2_experiment::FamilyError,
};
use nsbu_solver::{diagnostics::balances::BalanceSample, domain::TickClock, SolverError};
pub use plan::{V2BalanceBounds, V2BalancePlan, V2BalanceWork};

/// Six complete balances retaining the reconstruction identity and real node origins.
#[derive(Debug, Clone, Copy)]
pub struct V2BalanceSample {
    reconstruction: ProbeSample,
    branches: [BalanceSample; 6],
}
impl V2BalanceSample {
    /// Exact reconstructed physical clock.
    pub fn clock(self) -> TickClock {
        self.reconstruction.clock()
    }
    /// Original complete probe record and accepted-node origins.
    pub fn reconstruction(&self) -> &ProbeSample {
        &self.reconstruction
    }
    /// Complete branch balances in exact-v2 family order.
    pub fn branches(&self) -> &[BalanceSample; 6] {
        &self.branches
    }
}

/// Six independently forced balance observers with one report boundary.
pub struct V2Balances<'a> {
    plan: V2BalancePlan<'a>,
    children: [V2Observer; 6],
    charged: V2BalanceWork,
    next: usize,
    failed: bool,
}
impl<'a> V2Balances<'a> {
    /// Allocate every child only after complete joint admission.
    pub fn new(plan: V2BalancePlan<'a>) -> Result<Self, SolverError> {
        let values = plan.sources.map(|source| {
            V2Observer::new(
                source,
                plan.force,
                plan.bounds.work.attempts,
                plan.bounds.storage_bytes,
            )
        });
        let [a, b, c, d, e, f] = values;
        Ok(Self {
            plan,
            children: [a?, b?, c?, d?, e?, f?],
            charged: V2BalanceWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining complete report attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Charged whole-report work, including refused attempts.
    pub fn charged_work(&self) -> V2BalanceWork {
        self.charged
    }
    /// Actual child ledgers, distinct from aggregate refused-attempt charges.
    pub fn child_work(&self) -> [BalanceObserverWork; 6] {
        self.children.each_ref().map(V2Observer::consumption)
    }
    /// Next exact clock required by the immutable probe manifest.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// Whether a prior failed attempt permanently terminated this consumer.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Bind and publish one complete six-branch balance report.
    pub fn measure(&mut self, family: &ProbeFamily<'_>) -> Result<V2BalanceSample, FamilyError> {
        self.charge()?;
        let result = self
            .bind(family)
            .and_then(|sample| self.compute(family, sample));
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
            self.failed = true;
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        let one = self.plan.per_attempt;
        self.charged.attempts += 1;
        self.charged.children.samples += one.children.samples;
        self.charged.children.work_units += one.children.work_units;
        self.charged.children.scalar_transforms += one.children.scalar_transforms;
        self.charged.binding_comparisons += one.binding_comparisons;
        Ok(())
    }
    fn bind(&self, family: &ProbeFamily<'_>) -> Result<ProbeSample, FamilyError> {
        let expected = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        let sample = family.current.ok_or(FamilyError::InvalidFamily)?;
        if family.failed
            || family.plan.identity != self.plan.probes.identity
            || sample.identity() != self.plan.probes.identity
            || sample.clock() != expected
        {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(sample)
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        reconstruction: ProbeSample,
    ) -> Result<V2BalanceSample, FamilyError> {
        let values = std::array::from_fn(|index| {
            let fields = family.fields(index).ok_or(FamilyError::InvalidFamily)?;
            self.children[index]
                .sample_probe(fields.domain, fields.clock, fields.value)
                .map_err(FamilyError::from)
        });
        let [a, b, c, d, e, f] = values;
        Ok(V2BalanceSample {
            reconstruction,
            branches: [a?, b?, c?, d?, e?, f?],
        })
    }
}
