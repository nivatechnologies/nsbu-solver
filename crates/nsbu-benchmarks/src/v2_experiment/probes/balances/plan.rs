//! Joint admission for exact-v2 reconstructed balance reports.
use super::{V2BalanceSample, V2Balances};
use crate::{
    runtime_force::ForceSettings,
    smooth_observer::{v2::V2Observer, BalanceObserverWork},
    v2_experiment::{probes::ProbePlan, FamilyError},
};
use nsbu_solver::{domain::Domain, SolverError};

/// Finite work for complete six-branch report attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct V2BalanceWork {
    /// Charged report attempts.
    pub attempts: usize,
    /// Six independent observer allowances.
    pub children: BalanceObserverWork,
    /// Exact identity and clock comparisons.
    pub binding_comparisons: usize,
}
/// Balance-only and simultaneous owner bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V2BalanceBounds {
    /// Six observers and bounded report metadata.
    pub storage_bytes: usize,
    /// Probe owners/scratch plus balance consumer.
    pub joint_storage_bytes: usize,
    /// Complete maximum-attempt work.
    pub work: V2BalanceWork,
}
/// Immutable balance admission bound to one exact-v2 probe plan.
#[derive(Debug, Clone, Copy)]
pub struct V2BalancePlan<'a> {
    pub(super) probes: ProbePlan<'a>,
    pub(super) sources: [Domain; 6],
    pub(super) force: ForceSettings,
    pub(super) bounds: V2BalanceBounds,
    pub(super) per_attempt: V2BalanceWork,
}
impl<'a> V2BalancePlan<'a> {
    /// Admit all six fixed-force observers and complete attempts before allocation.
    pub fn new(
        probes: ProbePlan<'a>,
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if maximum_attempts < probes.tested_times().as_slice().len() {
            return Err(FamilyError::InvalidFamily);
        }
        let sources = probes.branches.map(|branch| branch.resources().domain());
        let force = probes.family.settings.force;
        validate_force_profile(force, sources)?;
        let (storage_bytes, per_attempt) = reservation(probes, sources, force)?;
        let joint_storage_bytes = add(storage_bytes, probes.bounds().joint_storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            probes,
            sources,
            force,
            bounds: V2BalanceBounds {
                storage_bytes,
                joint_storage_bytes,
                work: scale(per_attempt, maximum_attempts)?,
            },
            per_attempt,
        })
    }
    /// Complete simultaneous storage and finite work.
    pub fn bounds(self) -> V2BalanceBounds {
        self.bounds
    }
    /// Underlying immutable exact-v2 reconstruction manifest.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
}
fn validate_force_profile(force: ForceSettings, sources: [Domain; 6]) -> Result<(), SolverError> {
    let samples = force.double_grid()?.samples.dimensions();
    for source in sources {
        let required =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?
            .layout()
            .dimensions();
        if required
            .into_iter()
            .zip(samples)
            .any(|(need, have)| need > have)
        {
            return Err(SolverError::InvalidDomain);
        }
    }
    Ok(())
}
fn reservation(
    probes: ProbePlan<'_>,
    sources: [Domain; 6],
    force: ForceSettings,
) -> Result<(usize, V2BalanceWork), SolverError> {
    let mut bytes = add(
        std::mem::size_of::<V2Balances<'_>>(),
        add(std::mem::size_of::<V2BalanceSample>(), 64 * 8)?,
    )?;
    let mut one = V2BalanceWork {
        attempts: 1,
        ..V2BalanceWork::default()
    };
    for source in sources {
        let limits = V2Observer::limits(source, force, 1)?;
        bytes = add(bytes, limits.storage_bytes)?;
        one.children.samples = add(one.children.samples, 1)?;
        one.children.work_units = add(one.children.work_units, limits.work_units)?;
        one.children.scalar_transforms =
            add(one.children.scalar_transforms, limits.scalar_transforms)?;
    }
    one.binding_comparisons = add(probes.tested_times().as_slice().len(), 4)?;
    Ok((bytes, one))
}
fn scale(work: V2BalanceWork, n: usize) -> Result<V2BalanceWork, SolverError> {
    Ok(V2BalanceWork {
        attempts: n,
        children: BalanceObserverWork {
            samples: mul(work.children.samples, n)?,
            work_units: mul(work.children.work_units, n)?,
            scalar_transforms: mul(work.children.scalar_transforms, n)?,
        },
        binding_comparisons: mul(work.binding_comparisons, n)?,
    })
}
pub(super) fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
pub(super) fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
