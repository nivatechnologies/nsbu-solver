//! Admit bounded lookahead, full-band interpolation scratch and every exact probe before allocation.
use super::ProbeFamily;
use crate::smooth_experiment::{FamilyBounds, FamilyError, FamilyPlan};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, hermite::HermiteWeights},
    domain::TickClock,
    verification::times::TestedTimes,
    SolverError,
};

/// Extra finite interpolation/comparison work; integration retains its complete original ledger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeWork {
    /// Attempted complete six-branch probe records.
    pub attempts: usize,
    /// Conservative weighted retained-coefficient visits, excluding integration and FFT internals.
    pub weighted_visits: usize,
}
/// Complete numerical-owner reservation, with caller-owned probe manifests/output separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeBounds {
    /// Original six from-rest trajectory owners and all their finite work.
    pub family: FamilyBounds,
    /// All owners, full-band value/derivative scratch and fixed report/header allowance.
    pub joint_storage_bytes: usize,
    /// Maximum complete report work; failed attempts retain their charge.
    pub work: ProbeWork,
    /// Number of exact Hermite geometry/weight checks performed during admission.
    pub admission_geometry_checks: usize,
}
/// Immutable arbitrary-time sampling over unchanged fixed-step independent trajectories.
#[derive(Debug, Clone, Copy)]
pub struct ProbePlan<'a> {
    pub(super) family: FamilyPlan<'a>,
    pub(super) times: TestedTimes<'a>,
    pub(super) bounds: ProbeBounds,
    pub(super) visits_per_attempt: usize,
}
impl<'a> ProbePlan<'a> {
    /// Probe clocks may fall between accepted macro endpoints. Each branch looks ahead only
    /// far enough to obtain its own three-node accepted history, including two initial steps.
    /// The endpoint must accommodate that initial lookahead; no extrapolation is permitted.
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
        let (bytes, visits) = reservation(family)?;
        if bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        let admission_geometry_checks = geometry(family, times)?;
        Ok(Self {
            family,
            times,
            visits_per_attempt: visits,
            bounds: ProbeBounds {
                family: family.bounds(),
                joint_storage_bytes: bytes,
                work: ProbeWork {
                    attempts: maximum_attempts,
                    weighted_visits: mul(visits, maximum_attempts)?,
                },
                admission_geometry_checks,
            },
        })
    }
    /// All simultaneous numerical storage and finite traversal allowances.
    pub fn bounds(self) -> ProbeBounds {
        self.bounds
    }
    /// Exact immutable physical probe manifest; this does not claim control between samples.
    pub fn tested_times(self) -> TestedTimes<'a> {
        self.times
    }
    /// Original independent fixed-step evolution profile.
    pub fn family_plan(self) -> FamilyPlan<'a> {
        self.family
    }
}
fn geometry(family: FamilyPlan<'_>, times: TestedTimes<'_>) -> Result<usize, FamilyError> {
    let checks = mul(times.as_slice().len(), 6)?;
    for branch in family.branches {
        let step = branch.plan.configuration().limits.step_ticks;
        for &probe in times.as_slice() {
            let nodes = nodes(probe, step)?;
            if nodes[2].elapsed() > family.settings.endpoint {
                return Err(FamilyError::InvalidFamily);
            }
            HermiteWeights::at(nodes, probe)?;
        }
    }
    Ok(checks)
}
fn reservation(family: FamilyPlan<'_>) -> Result<(usize, usize), SolverError> {
    let mut bytes = family.bounds().storage_bytes;
    let mut visits = 0;
    for branch in family.branches {
        let n = branch.plan.resources().domain().layout().half_len();
        bytes = add(bytes, mul(n, 6 * 16)?)?;
        visits = add(visits, add(mul(n, 256)?, 512)?)?;
    }
    for (a, b) in super::super::PAIRS {
        let comparison = ComparisonPlan::new(
            family.branches[a].plan.resources().domain(),
            family.branches[b].plan.resources().domain(),
        )?;
        visits = add(visits, mul(comparison.work_units(), 2)?)?;
    }
    bytes = add(
        bytes,
        add(
            std::mem::size_of::<ProbeFamily<'_>>(),
            mul(std::mem::size_of::<super::ProbeSample>(), 4)?,
        )?,
    )?;
    Ok((bytes, visits))
}
pub(super) fn nodes(probe: TickClock, step: u128) -> Result<[TickClock; 3], SolverError> {
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
