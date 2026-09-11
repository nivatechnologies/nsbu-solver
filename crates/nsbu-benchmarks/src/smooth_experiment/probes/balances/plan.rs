//! Complete independent balance workspace admission for every streamed physical probe.
use super::{BalanceProbeSample, BalanceProbes};
use crate::{
    smooth_experiment::{probes::ProbePlan, FamilyError},
    smooth_observer::{BalanceObserver, BalanceObserverWork},
};
use nsbu_solver::{domain::ResourcePlan, SolverError};
/// Finite attempted-report work across all six independent conservative diagnostic paths.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BalanceProbeWork {
    /// Attempted complete reports; invalid current probes consume this allowance.
    pub attempts: usize,
    /// Conservative six-child work allowance, separate from actual child ledgers.
    pub children: BalanceObserverWork,
    /// Upper bound on exact manifest/probe equality comparisons.
    pub binding_comparisons: usize,
}
/// Joint numerical storage and complete charged work, with retained caller artifacts separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceProbeBounds {
    /// Six independent balance observers and complete aggregate report/header allowance.
    pub storage_bytes: usize,
    /// Simultaneous probe owners/interpolants and balance workspace reservation.
    pub joint_storage_bytes: usize,
    /// Finite complete attempted-report charges.
    pub work: BalanceProbeWork,
}
/// Immutable binding to the complete original six-trajectory physical probe profile.
#[derive(Debug, Clone, Copy)]
pub struct BalanceProbePlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) resources: [ResourcePlan; 6],
    pub(super) bounds: BalanceProbeBounds,
    pub(super) one: BalanceProbeWork,
}
impl<'a> BalanceProbePlan<'a> {
    /// No numerical allocation occurs until all six children and joint storage/work are admitted.
    pub fn new(
        probes: ProbePlan<'a>,
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < probes.tested_times().as_slice().len() {
            return Err(FamilyError::InvalidFamily);
        }
        let resources = probes
            .family_plan()
            .branches
            .map(|branch| branch.plan.resources());
        let mut bytes = add(
            std::mem::size_of::<BalanceProbes<'_>>(),
            mul(4, std::mem::size_of::<BalanceProbeSample>())?,
        )?;
        let mut one = BalanceProbeWork {
            attempts: 1,
            ..BalanceProbeWork::default()
        };
        for resource in resources {
            let limits = BalanceObserver::limits(resource.domain(), 1)?;
            BalanceObserver::validate_restored(
                resource,
                maximum_attempts,
                BalanceObserverWork::default(),
            )?;
            bytes = add(bytes, limits.storage_bytes)?;
            one.children.samples = add(one.children.samples, 1)?;
            one.children.work_units = add(one.children.work_units, limits.work_units)?;
            one.children.scalar_transforms =
                add(one.children.scalar_transforms, limits.scalar_transforms)?;
        }
        one.binding_comparisons = add(
            add(
                probes.tested_times().as_slice().len(),
                probes.family_plan().times.as_slice().len(),
            )?,
            1,
        )?;
        let joint_storage_bytes = add(bytes, probes.bounds().joint_storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let work = BalanceProbeWork {
            attempts: maximum_attempts,
            children: BalanceObserverWork {
                samples: mul(one.children.samples, maximum_attempts)?,
                work_units: mul(one.children.work_units, maximum_attempts)?,
                scalar_transforms: mul(one.children.scalar_transforms, maximum_attempts)?,
            },
            binding_comparisons: mul(one.binding_comparisons, maximum_attempts)?,
        };
        Ok(Self {
            probes,
            resources,
            one,
            bounds: BalanceProbeBounds {
                storage_bytes: bytes,
                joint_storage_bytes,
                work,
            },
        })
    }
    /// Full immutable simultaneous storage and finite-work declaration.
    pub fn bounds(self) -> BalanceProbeBounds {
        self.bounds
    }
    /// Original independently evolved owner and exact physical probe manifest.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
}
pub(super) fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
pub(super) fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
