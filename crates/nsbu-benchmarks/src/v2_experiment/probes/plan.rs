//! Admit six reconstructed owners, off-stage geometry, scratch and finite comparison work.
use super::ProbeFamily;
use crate::{
    v2_experiment::{FamilyBounds, FamilyError, FamilyPlan, PAIRS},
    v2_run::ReconstructedPlan,
};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, hermite::HermiteWeights},
    domain::TickClock,
    verification::times::TestedTimes,
    SolverError,
};

/// Complete work charged by off-stage probe attempts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeWork {
    /// Attempted complete six-branch records.
    pub attempts: usize,
    /// Conservative retained-coefficient visits for interpolation and comparison.
    pub weighted_visits: usize,
}

/// Aggregate owner, scratch and report admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeBounds {
    /// Original family admission retained for profile comparison.
    pub family: FamilyBounds,
    /// Six reconstruction-capable owners.
    pub owner_storage_bytes: usize,
    /// Six value/derivative scratch fields and coordinator metadata.
    pub scratch_storage_bytes: usize,
    /// Complete simultaneous owner and scratch storage.
    pub joint_storage_bytes: usize,
    /// Maximum complete probe work; failed attempts remain charged.
    pub work: ProbeWork,
    /// Exact three-node Hermite geometry checks completed during admission.
    pub admission_geometry_checks: usize,
}

/// Immutable exact-v2 off-stage probe admission.
#[derive(Debug, Clone, Copy)]
pub struct ProbePlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) branches: [ReconstructedPlan; 6],
    pub(super) times: TestedTimes<'a>,
    pub(super) bounds: ProbeBounds,
    pub(super) visits_per_attempt: usize,
    pub(super) identity: [u8; 32],
}
impl<'a> ProbePlan<'a> {
    /// Admit non-stage clocks, finite lookahead, all owners and complete report work.
    pub fn new(
        family: FamilyPlan<'a>,
        times: TestedTimes<'a>,
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, FamilyError> {
        if times.as_slice().first() != family.times.as_slice().first()
            || times.as_slice().last() != family.times.as_slice().last()
            || maximum_attempts < times.as_slice().len()
        {
            return Err(FamilyError::InvalidFamily);
        }
        let plans = family
            .branches
            .map(|branch| ReconstructedPlan::from_rest(branch.settings(), joint_cap));
        let [a, b, c, d, e, f] = plans;
        let branches = [a?, b?, c?, d?, e?, f?];
        let admission_geometry_checks = geometry(&branches, times, family.settings.endpoint)?;
        let (owner_storage_bytes, scratch_storage_bytes, visits_per_attempt) =
            reservation(&branches)?;
        let joint_storage_bytes = add(owner_storage_bytes, scratch_storage_bytes)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let work = ProbeWork {
            attempts: maximum_attempts,
            weighted_visits: mul(visits_per_attempt, maximum_attempts)?,
        };
        let identity = super::identity(family.identity(), times)?;
        Ok(Self {
            family,
            branches,
            times,
            bounds: ProbeBounds {
                family: family.bounds(),
                owner_storage_bytes,
                scratch_storage_bytes,
                joint_storage_bytes,
                work,
                admission_geometry_checks,
            },
            visits_per_attempt,
            identity,
        })
    }
    /// Complete aggregate storage and work bounds.
    pub fn bounds(self) -> ProbeBounds {
        self.bounds
    }
    /// Exact immutable off-stage clock manifest.
    pub fn tested_times(self) -> TestedTimes<'a> {
        self.times
    }
    /// Underlying unchanged exact-v2 family profile.
    pub fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
    /// Identity binding the exact-v2 family and complete probe manifest.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
}

fn geometry(
    branches: &[ReconstructedPlan; 6],
    times: TestedTimes<'_>,
    endpoint: u128,
) -> Result<usize, FamilyError> {
    let checks = mul(times.as_slice().len(), branches.len())?;
    for branch in branches {
        let step = branch.settings().configuration.limits.step_ticks;
        for &probe in times.as_slice() {
            let nodes = nodes(probe, step)?;
            if nodes[2].elapsed() > endpoint {
                return Err(FamilyError::InvalidFamily);
            }
            HermiteWeights::at(nodes, probe)?;
        }
    }
    Ok(checks)
}

fn reservation(branches: &[ReconstructedPlan; 6]) -> Result<(usize, usize, usize), SolverError> {
    let mut owners = 0;
    let mut scratch = add(std::mem::size_of::<ProbeFamily<'_>>(), 64 * 40)?;
    let mut visits = 0;
    for branch in branches {
        owners = add(owners, branch.resources().total())?;
        let n = branch.resources().domain().layout().half_len();
        scratch = add(scratch, add(mul(n, 6 * 16)?, 6 * 64)?)?;
        visits = add(visits, add(mul(n, 256)?, 512)?)?;
    }
    for (a, b) in PAIRS {
        let comparison = ComparisonPlan::new(
            branches[a].resources().domain(),
            branches[b].resources().domain(),
        )?;
        visits = add(visits, mul(comparison.work_units(), 2)?)?;
    }
    Ok((owners, scratch, visits))
}

pub(crate) fn nodes(probe: TickClock, step: u128) -> Result<[TickClock; 3], SolverError> {
    let count = probe.elapsed().div_ceil(step).max(2);
    let mut result = [probe; 3];
    for (index, clock) in result.iter_mut().enumerate() {
        let elapsed = (count - 2 + index as u128)
            .checked_mul(step)
            .ok_or(SolverError::SizeOverflow)?;
        let remaining = probe
            .target()
            .checked_sub(elapsed)
            .ok_or(SolverError::InvalidClock)?;
        *clock = TickClock::restore(probe.exponent(), probe.target(), elapsed, remaining)?;
    }
    Ok(result)
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
