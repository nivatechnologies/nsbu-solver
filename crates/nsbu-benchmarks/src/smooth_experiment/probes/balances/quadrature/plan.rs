//! Three strict physical-time refinements of Simpson quadrature on unchanged trajectory owners.
use super::{BalanceQuadrature, QuadratureReport};
use crate::smooth_experiment::{
    probes::balances::{
        plan::{add, mul},
        BalanceProbePlan,
    },
    FamilyError,
};
use nsbu_solver::{verification::times::TestedTimes, SolverError};
/// Complete additional resources and fixed attempted-update work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuadratureBounds {
    /// Entire numerical owners, interpolants, balance observers and quadrature report state.
    pub joint_storage_bytes: usize,
    /// Upper bound for all admission membership/nesting/Simpson geometry checks.
    pub admission_comparisons: usize,
    /// Conservative six-branch/three-level history update allowance across all attempts.
    pub history_updates: usize,
    /// Conservative complete integral-read allowance across all attempts.
    pub integral_reads: usize,
    /// Strictly decreasing maximum Simpson half-panel spans, in exact ticks.
    pub maximum_half_spans: [u128; 3],
}
/// Three immutable nested Simpson manifests; physical quadrature varies independently of H.
#[derive(Debug, Clone, Copy)]
pub struct QuadraturePlan<'a> {
    pub(super) balance: BalanceProbePlan<'a>,
    pub(super) times: [TestedTimes<'a>; 3],
    pub(super) bounds: QuadratureBounds,
}
impl<'a> QuadraturePlan<'a> {
    /// Admit all work before scanning caller manifests or allocating any numerical owner.
    pub fn new(
        balance: BalanceProbePlan<'a>,
        times: [TestedTimes<'a>; 3],
        maximum_comparisons: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        let owner = balance.probe_plan().tested_times().as_slice();
        let comparisons = admission_bound(times, owner.len())?;
        if comparisons > maximum_comparisons {
            return Err(SolverError::ResourceLimit.into());
        }
        let maximum_half_spans = geometry(times, owner)?;
        let extra = add(
            std::mem::size_of::<BalanceQuadrature<'_>>(),
            mul(4, std::mem::size_of::<QuadratureReport>())?,
        )?;
        let joint_storage_bytes = add(balance.bounds().joint_storage_bytes, extra)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let updates = mul(balance.bounds().work.attempts, 18)?;
        Ok(Self {
            balance,
            times,
            bounds: QuadratureBounds {
                joint_storage_bytes,
                admission_comparisons: comparisons,
                history_updates: updates,
                integral_reads: updates,
                maximum_half_spans,
            },
        })
    }
    /// Joint storage and separately bounded admission/attempt work.
    pub fn bounds(self) -> QuadratureBounds {
        self.bounds
    }
    /// Independently refined exact physical sample sets, not new integrator time steps.
    pub fn time_sets(self) -> [TestedTimes<'a>; 3] {
        self.times
    }
    /// Original six-branch balance consumer admission.
    pub fn balance_plan(self) -> BalanceProbePlan<'a> {
        self.balance
    }
}
fn admission_bound(times: [TestedTimes<'_>; 3], owner: usize) -> Result<usize, SolverError> {
    let [a, b, c] = times.map(|t| t.as_slice().len());
    // Fine-set membership, two linear nesting scans, endpoint checks and every Simpson half-span.
    add(
        add(mul(c, owner)?, add(b, c)?)?,
        add(4, mul(3, add(add(a, b)?, c)?)?)?,
    )
}
fn geometry(
    times: [TestedTimes<'_>; 3],
    owner: &[nsbu_solver::domain::TickClock],
) -> Result<[u128; 3], FamilyError> {
    let fine = times[2].as_slice();
    if !times[1].refines(times[0])
        || !times[2].refines(times[1])
        || fine.first() != owner.first()
        || fine.last() != owner.last()
    {
        return Err(FamilyError::InvalidFamily);
    }
    if !fine.iter().all(|clock| owner.contains(clock)) {
        return Err(FamilyError::InvalidFamily);
    }
    let mut spans = [0; 3];
    for (level, set) in times.into_iter().enumerate() {
        let clocks = set.as_slice();
        if clocks.len() < 3 || clocks.len() % 2 == 0 {
            return Err(FamilyError::InvalidFamily);
        }
        for i in (0..clocks.len() - 2).step_by(2) {
            spans[level] = spans[level].max(clocks[i + 1].elapsed() - clocks[i].elapsed());
            if clocks[i + 1].elapsed() - clocks[i].elapsed()
                != clocks[i + 2].elapsed() - clocks[i + 1].elapsed()
            {
                return Err(FamilyError::InvalidFamily);
            }
        }
    }
    if spans[1] >= spans[0] || spans[2] >= spans[1] {
        return Err(FamilyError::InvalidFamily);
    }
    Ok(spans)
}
