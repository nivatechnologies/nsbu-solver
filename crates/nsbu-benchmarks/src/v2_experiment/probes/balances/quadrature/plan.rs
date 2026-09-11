//! Three strict nested Simpson manifests over one exact-v2 balance consumer.
use super::{V2BalanceQuadrature, V2QuadratureReport};
use crate::v2_experiment::{
    probes::balances::{plan::add, plan::mul, V2BalancePlan},
    FamilyError,
};
use nsbu_solver::{verification::times::TestedTimes, SolverError};

/// Complete joint storage and finite quadrature work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V2QuadratureBounds {
    /// Probe owners, balance observers, histories and report metadata.
    pub joint_storage_bytes: usize,
    /// Maximum manifest membership, nesting and Simpson geometry checks.
    pub admission_comparisons: usize,
    /// Six-branch/three-level history update allowance.
    pub history_updates: usize,
    /// Complete integral-read allowance.
    pub integral_reads: usize,
    /// Strictly decreasing maximum Simpson half-panel spans.
    pub maximum_half_spans: [u128; 3],
}
/// Immutable three-level quadrature admission.
#[derive(Debug, Clone, Copy)]
pub struct V2QuadraturePlan<'a> {
    pub(super) balance: V2BalancePlan<'a>,
    pub(super) times: [TestedTimes<'a>; 3],
    pub(super) bounds: V2QuadratureBounds,
}
impl<'a> V2QuadraturePlan<'a> {
    /// Admit exact nested geometry, complete work and joint storage before allocation.
    pub fn new(
        balance: V2BalancePlan<'a>,
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
            std::mem::size_of::<V2BalanceQuadrature<'_>>(),
            mul(4, std::mem::size_of::<V2QuadratureReport>())?,
        )?;
        let joint_storage_bytes = add(balance.bounds().joint_storage_bytes, extra)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let work = mul(balance.bounds().work.attempts, 18)?;
        Ok(Self {
            balance,
            times,
            bounds: V2QuadratureBounds {
                joint_storage_bytes,
                admission_comparisons: comparisons,
                history_updates: work,
                integral_reads: work,
                maximum_half_spans,
            },
        })
    }
    /// Complete joint resource and quadrature-work declaration.
    pub fn bounds(self) -> V2QuadratureBounds {
        self.bounds
    }
    /// Coarse, middle and fine exact manifests.
    pub fn time_sets(self) -> [TestedTimes<'a>; 3] {
        self.times
    }
    /// Underlying exact-v2 balance consumer admission.
    pub fn balance_plan(self) -> V2BalancePlan<'a> {
        self.balance
    }
}
fn admission_bound(times: [TestedTimes<'_>; 3], owner: usize) -> Result<usize, SolverError> {
    let [a, b, c] = times.map(|set| set.as_slice().len());
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
        || !fine.iter().all(|clock| owner.contains(clock))
    {
        return Err(FamilyError::InvalidFamily);
    }
    let mut spans = [0; 3];
    for (level, set) in times.into_iter().enumerate() {
        let clocks = set.as_slice();
        if clocks.len() < 3 || clocks.len() % 2 == 0 {
            return Err(FamilyError::InvalidFamily);
        }
        for index in (0..clocks.len() - 2).step_by(2) {
            let left = clocks[index + 1].elapsed() - clocks[index].elapsed();
            let right = clocks[index + 2].elapsed() - clocks[index + 1].elapsed();
            spans[level] = spans[level].max(left);
            if left != right {
                return Err(FamilyError::InvalidFamily);
            }
        }
    }
    if spans[1] >= spans[0] || spans[2] >= spans[1] {
        return Err(FamilyError::InvalidFamily);
    }
    Ok(spans)
}
