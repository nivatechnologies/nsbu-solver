//! Complete admission for six independently evolved smooth comparison branches.
use super::{FamilyError, SmoothFamily};
use crate::{
    smooth_observer::reconstruction::ReconstructionObserver, smooth_run::ReconstructedPlan,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    verification::times::TestedTimes,
    SolverError,
};

/// Explicit smooth comparison settings, fixed before any branch is allocated.
#[derive(Debug, Clone, Copy)]
pub struct FamilySettings {
    /// Three strictly increasing cubic grids for full-band spatial comparison.
    pub grids: [usize; 3],
    /// Three strictly decreasing, nested positive macro-step tick counts.
    pub steps: [u128; 3],
    /// Unit-cube viscosity, shared by every mathematical problem instance.
    pub viscosity: f64,
    /// Exact endpoint measured from rest; every sample and step must align.
    pub endpoint: u128,
    /// Unchanged local integration tolerance on every branch.
    pub tolerances: Tolerances,
    /// Positive finite advective guard used by every branch.
    pub advective_limit: f64,
}
/// Separate finite storage and work allowances; work units are not a wall-clock estimate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FamilyBounds {
    /// All six complete run reservations and fixed family metadata; caller output is separate.
    pub storage_bytes: usize,
    /// Maximum integration RHS calls across all independently evolved branches.
    pub integration_calls: usize,
    /// Integration and independent diagnostic force-provider work units.
    pub provider_work_units: usize,
    /// Integration and diagnostic scalar transforms.
    pub scalar_transforms: usize,
    /// Independent reconstruction coefficient visits.
    pub reconstruction_modal_visits: usize,
    /// Full-band comparison work units across the entire supplied time manifest.
    pub comparison_work_units: usize,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Branch {
    pub(super) plan: ReconstructedPlan,
}
/// Borrowed exact time manifest and complete allocation-free branch admission.
#[derive(Debug, Clone, Copy)]
pub struct FamilyPlan<'a> {
    pub(super) branches: [Branch; 6],
    pub(super) times: TestedTimes<'a>,
    pub(super) bounds: FamilyBounds,
    pub(super) settings: FamilySettings,
}
impl<'a> FamilyPlan<'a> {
    /// Validate all six plans, synchronized times and aggregate capacity before allocating any run.
    pub fn new(
        settings: FamilySettings,
        times: TestedTimes<'a>,
        cap: usize,
    ) -> Result<Self, FamilyError> {
        validate_settings(settings, times)?;
        let clock = times.as_slice()[0];
        let [a, b, c] = settings.grids;
        let [h0, h1, h2] = settings.steps;
        let shapes = [
            (a, h2, Method::CoxMatthews),
            (b, h2, Method::CoxMatthews),
            (c, h2, Method::CoxMatthews),
            (c, h0, Method::CoxMatthews),
            (c, h1, Method::CoxMatthews),
            (c, h2, Method::HochbruckOstermann),
        ];
        let mut branches = [branch(settings, clock, shapes[0], cap)?; 6];
        for (slot, shape) in branches.iter_mut().zip(shapes).skip(1) {
            *slot = branch(settings, clock, shape, cap)?;
        }
        let bounds = reservations(&branches, times.as_slice().len())?;
        if bounds.storage_bytes > cap {
            return Err(FamilyError::Numerical(SolverError::ResourceLimit));
        }
        Ok(Self {
            branches,
            times,
            bounds,
            settings,
        })
    }
    // Shared private binding for all actual-state observation consumers.
    pub(super) fn require_sample(
        self,
        family: &SmoothFamily<'_>,
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
        if !self.same_profile(family.plan)
            || family
                .next
                .checked_sub(1)
                .and_then(|n| family.plan.times.as_slice().get(n))
                .copied()
                != Some(clock)
            || family
                .branches
                .iter()
                .any(|branch| branch.state().clock() != clock)
        {
            return Err(FamilyError::InvalidFamily);
        }
        Ok(clock)
    }
    // Shared exact binding for accepted-state and reconstructed-field consumers.
    pub(super) fn same_profile(self, other: FamilyPlan<'_>) -> bool {
        same_settings(self.settings, other.settings)
            && self.times.as_slice() == other.times.as_slice()
    }
    /// Checked aggregate reservation, including all independent observers.
    pub fn bounds(self) -> FamilyBounds {
        self.bounds
    }
    /// Frozen user settings; no tolerance is inferred from the resulting differences.
    pub fn settings(self) -> FamilySettings {
        self.settings
    }
}
fn validate_settings(settings: FamilySettings, times: TestedTimes<'_>) -> Result<(), FamilyError> {
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
    {
        return Err(FamilyError::InvalidFamily);
    }
    if times
        .as_slice()
        .last()
        .is_none_or(|t| t.elapsed() != settings.endpoint)
        || times
            .as_slice()
            .iter()
            .any(|t| !t.elapsed().is_multiple_of(settings.steps[0]))
    {
        return Err(FamilyError::InvalidFamily);
    }
    Ok(())
}
fn branch(
    settings: FamilySettings,
    clock: TickClock,
    (n, step, method): (usize, u128, Method),
    cap: usize,
) -> Result<Branch, FamilyError> {
    let count = usize::try_from(settings.endpoint / step).map_err(|_| SolverError::SizeOverflow)?;
    let configuration = Configuration {
        method,
        limits: RunLimits {
            endpoint: settings.endpoint,
            step_ticks: step,
            maximum_attempts: count,
        },
        tolerances: settings.tolerances,
    };
    let samples = count.checked_add(1).ok_or(SolverError::SizeOverflow)?;
    Ok(Branch {
        plan: ReconstructedPlan::from_rest(
            Domain::new([n; 3], [1.0; 3], settings.viscosity)?,
            clock,
            configuration,
            samples,
            settings.advective_limit,
            cap,
        )?,
    })
}
fn reservations(branches: &[Branch; 6], samples: usize) -> Result<FamilyBounds, FamilyError> {
    let mut total = FamilyBounds {
        storage_bytes: std::mem::size_of::<SmoothFamily<'_>>(),
        ..FamilyBounds::default()
    };
    for branch in branches {
        let plan = branch.plan;
        let observer =
            ReconstructionObserver::limits(plan.resources().domain(), plan.observer_samples())?;
        total.storage_bytes = add(total.storage_bytes, plan.resources().total())?;
        total.integration_calls = add(total.integration_calls, plan.integration_calls())?;
        total.provider_work_units = add(
            total.provider_work_units,
            add(plan.integration_work_units(), observer.work_units)?,
        )?;
        total.scalar_transforms = add(
            total.scalar_transforms,
            add(
                plan.integration_scalar_transforms(),
                observer.scalar_transforms,
            )?,
        )?;
        total.reconstruction_modal_visits =
            add(total.reconstruction_modal_visits, observer.modal_visits)?;
    }
    for (a, b) in super::PAIRS {
        let comparison = ComparisonPlan::new(
            branches[a].plan.resources().domain(),
            branches[b].plan.resources().domain(),
        )?;
        let work = comparison
            .work_units()
            .checked_mul(samples)
            .ok_or(SolverError::SizeOverflow)?;
        total.comparison_work_units = add(total.comparison_work_units, work)?;
    }
    Ok(total)
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}

// Exact numeric-policy words are compared; no tolerance or signed-zero normalization
// is introduced while binding the physical consumer to an actual family.
fn same_settings(left: FamilySettings, right: FamilySettings) -> bool {
    left.grids == right.grids
        && left.steps == right.steps
        && left.endpoint == right.endpoint
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.advective_limit.to_bits() == right.advective_limit.to_bits()
        && left.tolerances.absolute.map(f64::to_bits) == right.tolerances.absolute.map(f64::to_bits)
        && left.tolerances.relative.map(f64::to_bits) == right.tolerances.relative.map(f64::to_bits)
}
