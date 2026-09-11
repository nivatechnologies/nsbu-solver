//! Allocation-free admission of six independent exact-v2 from-rest branches.
use super::{identity, FamilyError, V2Family};
use crate::v2_run;
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, TickClock},
    experiment::control::Configuration,
    integrators::{method::Method, trajectory::RunLimits},
    verification::times::TestedTimes,
    SolverError,
};

/// Fixed settings shared by all six diagnostic exact-v2 from-rest branches.
#[derive(Debug, Clone, Copy)]
pub struct FamilySettings {
    /// Strictly increasing spatial grids.
    pub grids: [usize; 3],
    /// Strictly decreasing nested macro steps.
    pub steps: [u128; 3],
    /// Shared exact-v2 force sampling and worker selection.
    pub force: crate::runtime_force::ForceSettings,
    /// Endpoint tick count.
    pub endpoint: u128,
    /// Shared local integration tolerances.
    pub tolerances: nsbu_solver::integrators::indicator::Tolerances,
    /// Positive finite advective guard.
    pub advective_limit: f64,
}

/// Checked aggregate reservations for the six branches and comparisons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FamilyBounds {
    /// Aggregate branch and family reservation.
    pub storage_bytes: usize,
    /// Aggregate integration RHS calls.
    pub integration_calls: usize,
    /// Aggregate provider work units.
    pub provider_work_units: usize,
    /// Aggregate scalar transforms.
    pub scalar_transforms: usize,
    /// Canonical bytes visited by streaming identity construction.
    pub identity_bytes: usize,
    /// Aggregate complete comparison visits.
    pub comparison_work_units: usize,
}

/// Immutable borrowed admission for one complete family.
#[derive(Debug, Clone, Copy)]
pub struct FamilyPlan<'a> {
    pub(super) branches: [v2_run::Plan; 6],
    pub(super) times: TestedTimes<'a>,
    pub(super) settings: FamilySettings,
    pub(super) bounds: FamilyBounds,
    pub(super) identity: [u8; 32],
}

impl<'a> FamilyPlan<'a> {
    /// Admit all branches and checked comparison work before allocation.
    pub fn new(
        settings: FamilySettings,
        times: TestedTimes<'a>,
        cap: usize,
    ) -> Result<Self, FamilyError> {
        validate(settings, times)?;
        let initial = times.as_slice()[0];
        let [n0, n1, n2] = settings.grids;
        let [h0, h1, h2] = settings.steps;
        let shapes = [
            (n0, h2, Method::CoxMatthews),
            (n1, h2, Method::CoxMatthews),
            (n2, h2, Method::CoxMatthews),
            (n2, h0, Method::CoxMatthews),
            (n2, h1, Method::CoxMatthews),
            (n2, h2, Method::HochbruckOstermann),
        ];
        let branches = shapes
            .map(|shape| branch(settings, initial, shape, cap))
            .map(|result| result.map_err(FamilyError::from));
        let [b0, b1, b2, b3, b4, b5] = branches;
        let branches = [b0?, b1?, b2?, b3?, b4?, b5?];
        let bounds = reservations(&branches, times.as_slice().len())?;
        if bounds.storage_bytes > cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            branches,
            times,
            settings,
            bounds,
            identity: identity::compute(settings, times)?,
        })
    }
    /// Settings bound to this admission.
    pub fn settings(self) -> FamilySettings {
        self.settings
    }
    /// Borrowed exact tested-time manifest.
    pub fn times(self) -> TestedTimes<'a> {
        self.times
    }
    /// Aggregate bounded storage and work.
    pub fn bounds(self) -> FamilyBounds {
        self.bounds
    }
    /// Return one branch's immutable run plan.
    pub fn branch_plan(self, index: usize) -> Option<v2_run::Plan> {
        self.branches.get(index).copied()
    }
    /// Canonical SHA-256 family identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }

    pub(crate) fn require_sample(
        self,
        family: &V2Family<'_>,
        next: usize,
    ) -> Result<TickClock, FamilyError> {
        let clock = self
            .times
            .as_slice()
            .get(next)
            .copied()
            .ok_or(FamilyError::InvalidFamily)?;
        if family.failed {
            return Err(FamilyError::Terminated);
        }
        if self.identity != family.plan.identity
            || family.next != next + 1
            || family.plan.times.as_slice().get(next).copied() != Some(clock)
            || family
                .branches
                .iter()
                .any(|branch| branch.state().clock() != clock)
        {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(clock)
    }
}

fn validate(settings: FamilySettings, times: TestedTimes<'_>) -> Result<(), FamilyError> {
    if !(settings.grids[0] < settings.grids[1]
        && settings.grids[1] < settings.grids[2]
        && settings.steps[0] > settings.steps[1]
        && settings.steps[1] > settings.steps[2])
    {
        return Err(FamilyError::InvalidFamily);
    }
    if settings.steps[2] == 0
        || !settings.steps[0].is_multiple_of(settings.steps[1])
        || !settings.steps[1].is_multiple_of(settings.steps[2])
        || settings.endpoint == 0
    {
        return Err(FamilyError::InvalidFamily);
    }
    let first = times.as_slice().first().ok_or(FamilyError::InvalidFamily)?;
    if crate::time::BenchmarkTime::new(*first).is_err()
        || times
            .as_slice()
            .last()
            .is_none_or(|clock| clock.elapsed() != settings.endpoint)
        || times.as_slice().iter().any(|clock| {
            crate::time::BenchmarkTime::new(*clock).is_err()
                || !clock.elapsed().is_multiple_of(settings.steps[0])
        })
    {
        return Err(FamilyError::InvalidFamily);
    }
    settings.tolerances.validate().map_err(FamilyError::from)?;
    Ok(())
}

fn branch(
    settings: FamilySettings,
    initial: TickClock,
    (grid, step, method): (usize, u128, Method),
    cap: usize,
) -> Result<v2_run::Plan, SolverError> {
    let count = usize::try_from(settings.endpoint / step).map_err(|_| SolverError::SizeOverflow)?;
    let config = Configuration {
        method,
        limits: RunLimits {
            endpoint: settings.endpoint,
            step_ticks: step,
            maximum_attempts: count,
        },
        tolerances: settings.tolerances,
    };
    v2_run::Plan::from_rest(
        v2_run::Settings {
            domain: Domain::new([grid; 3], [1.0; 3], 1.0)?,
            force: settings.force,
            initial_clock: initial,
            configuration: config,
            advective_limit: settings.advective_limit,
        },
        cap,
    )
}

fn reservations(branches: &[v2_run::Plan; 6], samples: usize) -> Result<FamilyBounds, FamilyError> {
    let mut total = FamilyBounds {
        storage_bytes: std::mem::size_of::<super::V2Family<'_>>(),
        identity_bytes: identity::encoded_len(samples)?,
        ..FamilyBounds::default()
    };
    for plan in branches {
        let limits = plan.integration_limits();
        let observer = plan.observer_limits();
        total.storage_bytes = add(total.storage_bytes, plan.resources().total())?;
        total.integration_calls = add(total.integration_calls, limits[0])?;
        total.provider_work_units = add(
            total.provider_work_units,
            add(limits[1], observer.work_units)?,
        )?;
        total.scalar_transforms = add(
            total.scalar_transforms,
            add(limits[2], observer.scalar_transforms)?,
        )?;
    }
    for (a, b) in super::PAIRS {
        let work = ComparisonPlan::new(
            branches[a].resources().domain(),
            branches[b].resources().domain(),
        )?
        .work_units()
        .checked_mul(samples)
        .ok_or(SolverError::SizeOverflow)?;
        total.comparison_work_units = add(total.comparison_work_units, work)?;
    }
    Ok(total)
}
fn add(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_add(right).ok_or(SolverError::SizeOverflow)
}
