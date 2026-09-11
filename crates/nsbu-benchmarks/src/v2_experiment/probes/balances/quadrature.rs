//! Transactional three-level Simpson histories over exact-v2 balance reports.
mod plan;
use super::{V2BalanceSample, V2Balances};
use crate::v2_experiment::{probes::ProbeFamily, FamilyError};
use nsbu_solver::{
    diagnostics::{history::BalanceHistory, quadrature::BalanceIntegral},
    SolverError,
};
pub use plan::{V2QuadratureBounds, V2QuadraturePlan};

/// Complete coarse, middle and fine Simpson reductions for six unchanged owners.
#[derive(Debug, Clone, Copy)]
pub struct V2QuadratureReport {
    endpoint: crate::v2_experiment::probes::ProbeSample,
    integrals: [[BalanceIntegral; 6]; 3],
    counts: [usize; 3],
}
impl V2QuadratureReport {
    /// Final exact probe record; callers retain earlier emitted source records.
    pub fn endpoint(&self) -> &crate::v2_experiment::probes::ProbeSample {
        &self.endpoint
    }
    /// Coarse, middle and fine integrals in exact-v2 branch order.
    pub fn integrals(&self) -> &[[BalanceIntegral; 6]; 3] {
        &self.integrals
    }
    /// Complete sample counts for the three nested manifests.
    pub fn counts(self) -> [usize; 3] {
        self.counts
    }
}

/// Fixed-storage Simpson histories around independently owned exact-v2 observers.
pub struct V2BalanceQuadrature<'a> {
    plan: V2QuadraturePlan<'a>,
    consumer: V2Balances<'a>,
    histories: [[Option<BalanceHistory>; 6]; 3],
    counts: [usize; 3],
    report: Option<V2QuadratureReport>,
    failed: bool,
}
impl<'a> V2BalanceQuadrature<'a> {
    /// Allocate only the already admitted balance observers.
    pub fn new(plan: V2QuadraturePlan<'a>) -> Result<Self, SolverError> {
        Ok(Self {
            plan,
            consumer: V2Balances::new(plan.balance)?,
            histories: [[None; 6]; 3],
            counts: [0; 3],
            report: None,
            failed: false,
        })
    }
    /// Complete final report, absent before completion and after any failed attempt.
    pub fn report(&self) -> Option<&V2QuadratureReport> {
        self.report.as_ref()
    }
    /// Committed source counts, including an unfinished midpoint.
    pub fn sample_counts(&self) -> [usize; 3] {
        self.counts
    }
    /// Charged balance work, including refused attempts.
    pub fn charged_work(&self) -> super::V2BalanceWork {
        self.consumer.charged_work()
    }
    /// Admitted history-update and integral-read charges for attempted reports.
    pub fn charged_quadrature_work(&self) -> [usize; 2] {
        [18 * self.consumer.charged_work().attempts; 2]
    }
    /// Whether history or its child balance consumer has terminated.
    pub fn is_terminated(&self) -> bool {
        self.failed || self.consumer.is_terminated()
    }
    /// Measure balances, privately update all affected histories, then commit together.
    pub fn measure(&mut self, family: &ProbeFamily<'_>) -> Result<V2BalanceSample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        self.report = None;
        let sample = self.consumer.measure(family)?;
        match self.pending(sample) {
            Ok((histories, counts, report)) => {
                self.histories = histories;
                self.counts = counts;
                self.report = report;
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    #[cfg(test)]
    pub(super) fn inject_invalid_midpoint_for_test(
        &mut self,
        clock: nsbu_solver::domain::TickClock,
    ) {
        self.histories[0][3] = Some(
            self.histories[0][3]
                .unwrap()
                .with_sample(
                    clock,
                    nsbu_solver::diagnostics::balances::BalanceSample::REST,
                )
                .unwrap(),
        );
    }
    fn pending(&self, sample: V2BalanceSample) -> Result<Pending, FamilyError> {
        let mut histories = self.histories;
        let mut counts = self.counts;
        for (level, set) in self.plan.times.into_iter().enumerate() {
            if set.as_slice().get(counts[level]).copied() != Some(sample.clock()) {
                continue;
            }
            for (branch, value) in sample.branches().iter().copied().enumerate() {
                histories[level][branch] = Some(match histories[level][branch] {
                    None => BalanceHistory::new(sample.clock(), value, set.as_slice().len())?,
                    Some(history) => history.with_sample(sample.clock(), value)?,
                });
            }
            counts[level] += 1;
        }
        let report = self.complete_report(sample, histories, counts)?;
        Ok((histories, counts, report))
    }
    fn complete_report(
        &self,
        sample: V2BalanceSample,
        histories: [[Option<BalanceHistory>; 6]; 3],
        counts: [usize; 3],
    ) -> Result<Option<V2QuadratureReport>, FamilyError> {
        if Some(&sample.clock()) != self.plan.times[2].as_slice().last() {
            return Ok(None);
        }
        for (count, set) in counts.into_iter().zip(self.plan.times) {
            if count != set.as_slice().len() {
                return Err(FamilyError::InvalidFamily);
            }
        }
        Ok(Some(V2QuadratureReport {
            endpoint: *sample.reconstruction(),
            integrals: complete(histories)?,
            counts,
        }))
    }
}
type Pending = (
    [[Option<BalanceHistory>; 6]; 3],
    [usize; 3],
    Option<V2QuadratureReport>,
);
fn complete(
    histories: [[Option<BalanceHistory>; 6]; 3],
) -> Result<[[BalanceIntegral; 6]; 3], SolverError> {
    let level = |values: [Option<BalanceHistory>; 6]| {
        let [a, b, c, d, e, f] =
            values.map(|value| value.ok_or(SolverError::InvalidPayload)?.integral());
        Ok::<_, SolverError>([a?, b?, c?, d?, e?, f?])
    };
    let [a, b, c] = histories.map(level);
    Ok([a?, b?, c?])
}
