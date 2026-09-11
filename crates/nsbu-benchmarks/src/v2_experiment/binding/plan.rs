//! Joint admission for bounded accepted-node equality binding.
use super::{NodeBindingSample, NodeBindingWorkspace};
use crate::v2_experiment::{probes::ProbePlan, FamilyError, FamilyPlan};
use nsbu_solver::SolverError;

/// Worst-case accepted-node binding work, charged before request validation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NodeBindingWork {
    /// Complete or refused binding requests.
    pub attempts: usize,
    /// Comparisons against at most three retained node clocks per branch.
    pub node_lookups: usize,
    /// Velocity coefficient visits across all six branch pairs.
    pub coefficient_visits: usize,
}

/// Binder storage, aggregate two-family storage and finite work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeBindingBounds {
    /// Fixed binder and retained-report storage.
    pub storage_bytes: usize,
    /// One ordinary family, one probe family and binder storage.
    pub joint_storage_bytes: usize,
    /// Total worst-case request work.
    pub work: NodeBindingWork,
}

/// Immutable identity, accepted-clock manifest and resource policy.
#[derive(Debug, Clone, Copy)]
pub struct NodeBindingPlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) probes: ProbePlan<'a>,
    pub(super) bounds: NodeBindingBounds,
    pub(super) per_attempt: NodeBindingWork,
}
impl<'a> NodeBindingPlan<'a> {
    /// Admit two owners, fixed report retention and every possible coefficient visit.
    pub fn new(
        family: FamilyPlan<'a>,
        probes: ProbePlan<'a>,
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if family.identity() != probes.family_plan().identity()
            || maximum_attempts < family.times().as_slice().len()
        {
            return Err(FamilyError::InvalidFamily);
        }
        let per_attempt = work(family)?;
        let reports = std::mem::size_of::<NodeBindingSample>()
            .checked_mul(2)
            .ok_or(SolverError::SizeOverflow)?;
        let views = std::mem::size_of::<crate::v2_experiment::probes::ProbeAcceptedNode<'_>>()
            .checked_mul(6)
            .ok_or(SolverError::SizeOverflow)?;
        let storage_bytes = std::mem::size_of::<NodeBindingWorkspace<'_>>()
            .checked_add(reports)
            .and_then(|n| n.checked_add(views))
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = family
            .bounds()
            .storage_bytes
            .checked_add(probes.bounds().joint_storage_bytes)
            .and_then(|n| n.checked_add(storage_bytes))
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            family,
            probes,
            bounds: NodeBindingBounds {
                storage_bytes,
                joint_storage_bytes,
                work: scale(per_attempt, maximum_attempts)?,
            },
            per_attempt,
        })
    }
    /// Complete storage and work bounds.
    pub fn bounds(self) -> NodeBindingBounds {
        self.bounds
    }
    /// Underlying ordinary-family plan.
    pub fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
    /// Independently owned probe-family plan.
    pub fn probe_plan(self) -> ProbePlan<'a> {
        self.probes
    }
}

fn work(family: FamilyPlan<'_>) -> Result<NodeBindingWork, SolverError> {
    let visits = (0..6).try_fold(0usize, |total, branch| {
        let n = family
            .branch_plan(branch)
            .ok_or(SolverError::InvalidPayload)?
            .resources()
            .domain()
            .layout()
            .half_len();
        total
            .checked_add(n.checked_mul(3).ok_or(SolverError::SizeOverflow)?)
            .ok_or(SolverError::SizeOverflow)
    })?;
    Ok(NodeBindingWork {
        attempts: 1,
        node_lookups: 18,
        coefficient_visits: visits.checked_mul(2).ok_or(SolverError::SizeOverflow)?,
    })
}
fn scale(work: NodeBindingWork, attempts: usize) -> Result<NodeBindingWork, SolverError> {
    Ok(NodeBindingWork {
        attempts,
        node_lookups: work
            .node_lookups
            .checked_mul(attempts)
            .ok_or(SolverError::SizeOverflow)?,
        coefficient_visits: work
            .coefficient_visits
            .checked_mul(attempts)
            .ok_or(SolverError::SizeOverflow)?,
    })
}
