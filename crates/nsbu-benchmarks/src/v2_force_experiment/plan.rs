//! Allocation-free admission for three fixed-trajectory, varying-force-grid runs.
use super::{identity, ForceFamily, ForceFamilyError};
use crate::{runtime_force::ForceSettings, v2_run};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    verification::times::TestedTimes,
    SolverError,
};

/// One physical and integration policy with three nested force sample sizes.
#[derive(Debug, Clone, Copy)]
pub struct ForceFamilySettings {
    /// Fixed cubic retained velocity grid.
    pub grid: usize,
    /// Strictly increasing divisible cubic force grids M0, M1 and M2.
    pub force_grids: [usize; 3],
    /// Fixed provider worker count.
    pub workers: usize,
    /// Fixed macro step in exact ticks.
    pub step_ticks: u128,
    /// Fixed integration method.
    pub method: Method,
    /// Fixed endpoint tick count.
    pub endpoint: u128,
    /// Fixed local integration tolerances.
    pub tolerances: Tolerances,
    /// Fixed positive finite advective guard.
    pub advective_limit: f64,
}

/// Aggregate storage, attempts and finite work for the complete family.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForceFamilyBounds {
    /// All three run owners plus fixed family metadata.
    pub storage_bytes: usize,
    /// Total admitted step attempts across all branches.
    pub attempts: usize,
    /// Total integration RHS calls.
    pub integration_calls: usize,
    /// Total prescribed-force and observer work units.
    pub provider_work_units: usize,
    /// Total integration and observer scalar transforms.
    pub scalar_transforms: usize,
    /// Complete full-band comparison visits over the manifest.
    pub comparison_work_units: usize,
    /// Canonical bytes visited by streaming identity construction.
    pub identity_bytes: usize,
}

/// Immutable aggregate admission borrowing an exact synchronized manifest.
#[derive(Debug, Clone, Copy)]
pub struct ForceFamilyPlan<'a> {
    pub(super) branches: [v2_run::Plan; 3],
    pub(super) times: TestedTimes<'a>,
    settings: ForceFamilySettings,
    bounds: ForceFamilyBounds,
    identity: [u8; 32],
}
impl<'a> ForceFamilyPlan<'a> {
    /// Validate the complete family and cap before allocating a numerical owner.
    pub fn new(
        settings: ForceFamilySettings,
        times: TestedTimes<'a>,
        cap: usize,
    ) -> Result<Self, ForceFamilyError> {
        validate(settings, times)?;
        let initial = times.as_slice()[0];
        let branch = |force_grid| branch(settings, initial, force_grid, cap);
        let [a, b, c] = settings.force_grids.map(branch);
        let branches = [a?, b?, c?];
        let bounds = reservation(&branches, times.as_slice().len())?;
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
    /// Frozen physical, integration and force-grid settings.
    pub fn settings(self) -> ForceFamilySettings {
        self.settings
    }
    /// Exact synchronized sample manifest.
    pub fn times(self) -> TestedTimes<'a> {
        self.times
    }
    /// Complete aggregate reservation and work bounds.
    pub fn bounds(self) -> ForceFamilyBounds {
        self.bounds
    }
    /// One immutable child run plan.
    pub fn branch_plan(self, index: usize) -> Option<v2_run::Plan> {
        self.branches.get(index).copied()
    }
    /// Canonical SHA-256 over every admitted setting and manifest clock.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Require a complete synchronized publication from the matching family owner.
    pub(crate) fn require_sample(
        self,
        family: &ForceFamily<'_>,
        next: usize,
    ) -> Result<TickClock, ForceFamilyError> {
        let clock = self
            .times
            .as_slice()
            .get(next)
            .copied()
            .ok_or(ForceFamilyError::InvalidFamily)?;
        if family.failed
            || family.plan.identity != self.identity
            || family.next != next + 1
            || family
                .branches
                .iter()
                .any(|branch| branch.state().clock() != clock)
        {
            return Err(ForceFamilyError::InvalidFamily);
        }
        Ok(clock)
    }
}

fn validate(settings: ForceFamilySettings, times: TestedTimes<'_>) -> Result<(), ForceFamilyError> {
    validate_force_grids(settings.force_grids)?;
    validate_schedule(settings, times)?;
    Domain::new([settings.grid; 3], [1.0; 3], 1.0)?;
    for grid in settings.force_grids {
        Layout::new([grid; 3])?;
    }
    settings.tolerances.validate()?;
    Ok(())
}

fn validate_force_grids(force_grids: [usize; 3]) -> Result<(), ForceFamilyError> {
    let [m0, m1, m2] = force_grids;
    if !(m0 < m1 && m1 < m2 && m1.is_multiple_of(m0) && m2.is_multiple_of(m1)) {
        return Err(ForceFamilyError::InvalidFamily);
    }
    Ok(())
}

fn validate_schedule(
    settings: ForceFamilySettings,
    times: TestedTimes<'_>,
) -> Result<(), ForceFamilyError> {
    if settings.step_ticks == 0
        || settings.endpoint == 0
        || !settings.endpoint.is_multiple_of(settings.step_ticks)
    {
        return Err(ForceFamilyError::InvalidFamily);
    }
    let clocks = times.as_slice();
    let first = clocks.first().ok_or(ForceFamilyError::InvalidFamily)?;
    if crate::time::BenchmarkTime::new(*first).is_err()
        || clocks
            .last()
            .is_none_or(|clock| clock.elapsed() != settings.endpoint)
        || clocks.iter().any(|clock| {
            crate::time::BenchmarkTime::new(*clock).is_err()
                || !clock.elapsed().is_multiple_of(settings.step_ticks)
        })
    {
        return Err(ForceFamilyError::InvalidFamily);
    }
    Ok(())
}

fn branch(
    settings: ForceFamilySettings,
    initial_clock: nsbu_solver::domain::TickClock,
    force_grid: usize,
    cap: usize,
) -> Result<v2_run::Plan, SolverError> {
    let attempts = usize::try_from(settings.endpoint / settings.step_ticks)
        .map_err(|_| SolverError::SizeOverflow)?;
    v2_run::Plan::from_rest(
        v2_run::Settings {
            domain: Domain::new([settings.grid; 3], [1.0; 3], 1.0)?,
            force: ForceSettings {
                samples: Layout::new([force_grid; 3])?,
                workers: settings.workers,
            },
            initial_clock,
            configuration: Configuration {
                limits: RunLimits {
                    endpoint: settings.endpoint,
                    step_ticks: settings.step_ticks,
                    maximum_attempts: attempts,
                },
                method: settings.method,
                tolerances: settings.tolerances,
            },
            advective_limit: settings.advective_limit,
        },
        cap,
    )
}

fn reservation(
    branches: &[v2_run::Plan; 3],
    samples: usize,
) -> Result<ForceFamilyBounds, SolverError> {
    let mut total = ForceFamilyBounds {
        storage_bytes: std::mem::size_of::<ForceFamily<'_>>(),
        identity_bytes: identity::encoded_len(samples)?,
        ..ForceFamilyBounds::default()
    };
    for plan in branches {
        let integration = plan.integration_limits();
        let observer = plan.observer_limits();
        total.storage_bytes = add(total.storage_bytes, plan.resources().total())?;
        total.attempts = add(
            total.attempts,
            plan.settings().configuration.limits.maximum_attempts,
        )?;
        total.integration_calls = add(total.integration_calls, integration[0])?;
        total.provider_work_units = add(
            total.provider_work_units,
            add(integration[1], observer.work_units)?,
        )?;
        total.scalar_transforms = add(
            total.scalar_transforms,
            add(integration[2], observer.scalar_transforms)?,
        )?;
    }
    let comparison = ComparisonPlan::new(
        branches[0].resources().domain(),
        branches[1].resources().domain(),
    )?
    .work_units()
    .checked_mul(2)
    .and_then(|work| work.checked_mul(samples))
    .ok_or(SolverError::SizeOverflow)?;
    total.comparison_work_units = comparison;
    Ok(total)
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
