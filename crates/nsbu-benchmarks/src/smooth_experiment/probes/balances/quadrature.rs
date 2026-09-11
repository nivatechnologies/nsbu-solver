//! Transactional three-level Simpson accumulation of actual reconstructed balance samples.
mod plan;
use super::{BalanceProbeSample, BalanceProbes};
use crate::smooth_experiment::{
    probes::{ProbeFamily, ProbeSample},
    FamilyError,
};
use nsbu_solver::{
    diagnostics::{history::BalanceHistory, quadrature::BalanceIntegral},
    SolverError,
};
pub use plan::{QuadratureBounds, QuadraturePlan};
/// Completed measured quadrature at all three physical-time resolutions for six unchanged owners.
/// Per-probe source records are emitted separately; this object alone is not full window provenance.
#[derive(Debug, Clone, Copy)]
pub struct QuadratureReport {
    endpoint: ProbeSample,
    integrals: [[BalanceIntegral; 6]; 3],
    counts: [usize; 3],
}
impl QuadratureReport {
    /// Last physical sample's original node origins; earlier records remain required evidence.
    pub fn endpoint(&self) -> &ProbeSample {
        &self.endpoint
    }
    /// Coarse/middle/fine quadratures, each retaining all six independent trajectory branches.
    pub fn integrals(&self) -> &[[BalanceIntegral; 6]; 3] {
        &self.integrals
    }
    /// Complete observed sample counts on the three original manifests.
    pub fn counts(self) -> [usize; 3] {
        self.counts
    }
}
/// Fixed-storage quadrature over separately computed conservative balance samples.
/// Changing physical-time sampling cannot modify the underlying integrator steps or states.
pub struct BalanceQuadrature<'a> {
    plan: QuadraturePlan<'a>,
    consumer: BalanceProbes<'a>,
    histories: [[Option<BalanceHistory>; 6]; 3],
    counts: [usize; 3],
    report: Option<QuadratureReport>,
    failed: bool,
}
impl<'a> BalanceQuadrature<'a> {
    /// Allocate only the previously admitted independent observers; all quadrature state is inline.
    pub fn new(plan: QuadraturePlan<'a>) -> Result<Self, SolverError> {
        Ok(Self {
            plan,
            consumer: BalanceProbes::new(plan.balance)?,
            histories: [[None; 6]; 3],
            counts: [0; 3],
            report: None,
            failed: false,
        })
    }
    /// Last complete three-level quadrature; absent until every endpoint and sample is present.
    pub fn report(&self) -> Option<&QuadratureReport> {
        self.report.as_ref()
    }
    /// Counts include initial rest and pending Simpson midpoints, never silently discarded.
    pub fn sample_counts(&self) -> [usize; 3] {
        self.counts
    }
    /// Original diagnostic charges, including refused requests and terminal numerical failures.
    pub fn charged_work(&self) -> super::BalanceProbeWork {
        self.consumer.charged_work()
    }
    /// Conservative history-update and complete integral-read charges for every attempted report.
    /// Bounds admit eighteen of each per attempt, including attempts refused before arithmetic.
    pub fn charged_quadrature_work(&self) -> [usize; 2] {
        [18 * self.consumer.charged_work().attempts; 2]
    }
    /// Permanent failure is distinct from an incomplete but still admissible sample sequence.
    pub fn is_terminated(&self) -> bool {
        self.failed || self.consumer.is_terminated()
    }
    /// Compute all balances, then atomically update every affected quadrature level.
    /// Malformed owners spend diagnostic work but retain history; numerical failures are terminal.
    pub fn measure(&mut self, family: &ProbeFamily<'_>) -> Result<BalanceProbeSample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
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
                self.report = None;
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
    fn pending(&self, sample: BalanceProbeSample) -> Result<Pending, FamilyError> {
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
        let report = if Some(&sample.clock()) == self.plan.times[2].as_slice().last() {
            for (count, set) in counts.into_iter().zip(self.plan.times) {
                if count != set.as_slice().len() {
                    return Err(FamilyError::InvalidFamily);
                }
            }
            Some(QuadratureReport {
                endpoint: *sample.reconstruction(),
                integrals: complete(histories)?,
                counts,
            })
        } else {
            None
        };
        Ok((histories, counts, report))
    }
}
type Pending = (
    [[Option<BalanceHistory>; 6]; 3],
    [usize; 3],
    Option<QuadratureReport>,
);
fn complete(
    histories: [[Option<BalanceHistory>; 6]; 3],
) -> Result<[[BalanceIntegral; 6]; 3], SolverError> {
    let level =
        |histories: [Option<BalanceHistory>; 6]| -> Result<[BalanceIntegral; 6], SolverError> {
            let [a, b, c, d, e, f] =
                histories.map(|h| h.ok_or(SolverError::InvalidPayload)?.integral());
            Ok([a?, b?, c?, d?, e?, f?])
        };
    let [a, b, c] = histories.map(level);
    Ok([a?, b?, c?])
}
