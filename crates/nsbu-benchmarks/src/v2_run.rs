//! Owned exact-v2 trajectories from rest, with transactional diagnostics and bounded work.
//!
//! The analytical reference is never assigned to the evolving state. This runtime exposes
//! the reviewed prescribed force and both integrators; it does not qualify a PDE window.
use crate::{
    runtime_force::{AttemptCacheWork, ForceSettings, RunForce},
    smooth_observer::{v2::V2Observer, BalanceObserverWork},
};
use nsbu_solver::{
    domain::{Domain, Epoch, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs, transaction::CandidateState},
    spectral::FftCatalog,
    SolverError,
};

pub mod archive;
mod reconstructed;

mod plan;
mod work;
pub use crate::runtime_force::IntegrationMode;
pub use plan::Plan;
pub use reconstructed::{ReconstructedPlan, ReconstructedRun};
pub use work::AttemptWork;

/// Exact configuration bound to one independently evolved trajectory.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    /// Unit-cube, unit-viscosity domain required by the reviewed case.
    pub domain: Domain,
    /// Integration force quadrature and persistent-worker selection.
    pub force: ForceSettings,
    /// Initial zero clock with exact target time 1/128.
    pub initial_clock: TickClock,
    /// Integrator, fixed tick interval, finite attempt count and local tolerances.
    pub configuration: Configuration,
    /// Positive advective admission limit; no rejected step is silently rescheduled.
    pub advective_limit: f64,
}

/// Diagnostic lineage, separate from scientific acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Constructed from exact zero in this process.
    InternalFromRest,
    /// Imported bytes passed consistency checks without establishing their provenance.
    ExternalUnverified,
}

/// A complete private runtime owner with read-only physical and diagnostic access.
pub struct Run {
    plan: Plan,
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<RunForce>,
    observer: V2Observer,
    history: RunHistory,
    work: Vec<AttemptWork>,
    origin: Origin,
}

impl Run {
    /// Allocate an admitted run from rest; every buffer and worker is reserved by `plan`.
    ///
    /// ```no_run
    /// use nsbu_benchmarks::{runtime_force::ForceSettings, v2_run::{Plan, Run, Settings}};
    /// use nsbu_solver::{
    ///     domain::{Domain, Layout, TickClock},
    ///     experiment::control::Configuration,
    ///     integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    /// };
    ///
    /// let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    /// let settings = Settings {
    ///     domain,
    ///     force: ForceSettings { samples: Layout::new([4; 3]).unwrap(), workers: 0 },
    ///     initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
    ///     configuration: Configuration {
    ///         limits: RunLimits { endpoint: 4096, step_ticks: 128, maximum_attempts: 32 },
    ///         method: Method::CoxMatthews,
    ///         tolerances: Tolerances { absolute: [1e-5, 1e-4], relative: [1e-5, 1e-5] },
    ///     },
    ///     advective_limit: 0.3,
    /// };
    /// let plan = Plan::from_rest(settings, 64 * 1024 * 1024).unwrap();
    /// let mut run = Run::from_rest(plan).unwrap();
    /// let first_outcome = run.step().unwrap();
    /// assert!(matches!(first_outcome, nsbu_solver::experiment::control::Outcome::Committed(_)));
    /// ```
    pub fn from_rest(plan: Plan) -> Result<Self, SolverError> {
        let settings = plan.settings();
        let state = SpectralState::from_rest(plan.resources(), settings.initial_clock, Epoch(0))?;
        let history = RunHistory::new(
            settings.initial_clock,
            settings.configuration,
            RunHistory::reservation(settings.configuration)?,
        )?;
        Self::restore_parts(
            plan,
            state,
            history,
            work::storage(plan)?,
            BalanceObserverWork::default(),
            Origin::InternalFromRest,
        )
    }

    /// Attempt one fixed interval. Inspect the outcome: an `Ok` can record a terminal refusal.
    ///
    /// Physical state and required balances commit together. Rejected and refused attempts
    /// retain their work charges while preserving the last committed physical state.
    pub fn step(&mut self) -> Result<Outcome, SolverError> {
        let generation = self.rhs.consumption_epoch();
        let before = self.observer.consumption();
        let outcome = recorded_step(
            &mut self.state,
            &mut self.candidate,
            &mut self.attempts,
            &mut self.rhs,
            &mut self.observer,
            &mut self.history,
        )?;
        let integration = if generation == self.rhs.consumption_epoch() {
            [0; 3]
        } else {
            self.rhs.consumption()
        };
        let after = self.observer.consumption();
        // Private counters are monotone, and admission reserved the complete attempt ledger.
        // No fallible operation or allocation follows the physical/history commit.
        self.work.push(AttemptWork {
            integration,
            observation: BalanceObserverWork {
                samples: after.samples - before.samples,
                work_units: after.work_units - before.work_units,
                scalar_transforms: after.scalar_transforms - before.scalar_transforms,
            },
        });
        Ok(outcome)
    }

    pub(super) fn restore_parts(
        plan: Plan,
        state: SpectralState,
        history: RunHistory,
        work: Vec<AttemptWork>,
        observer_work: BalanceObserverWork,
        origin: Origin,
    ) -> Result<Self, SolverError> {
        if work.capacity() < plan.settings().configuration.limits.maximum_attempts {
            return Err(SolverError::ResourceLimit);
        }
        work::validate(plan, &state, &history, &work, observer_work)?;
        let settings = plan.settings();
        let resources = plan.resources();
        let fft = FftCatalog::new(plan.fft_backend(), resources.classes()[4])?;
        let candidate = CandidateState::new(resources, settings.initial_clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(resources, settings.configuration.method)?;
        let force_limits = settings.force.integration_limits_with_fft_backend(
            settings.domain,
            plan.integration_mode(),
            plan.fft_backend(),
        )?;
        let force = settings.force.build_integration_with_catalog(
            settings.domain,
            plan.integration_mode(),
            &fft,
            force_limits.storage_bytes,
        )?;
        let rhs = SpectralRhs::new_with_catalog(
            settings.domain,
            force,
            settings.advective_limit,
            &fft,
            resources.classes()[5],
        )?;
        let observer = V2Observer::restore_with_catalog(
            settings.domain,
            settings.force,
            plan.observer_samples(),
            observer_work,
            &fft,
            plan.observer_limits().storage_bytes,
        )?;
        Ok(Self {
            plan,
            state,
            candidate,
            attempts,
            rhs,
            observer,
            history,
            work,
            origin,
        })
    }

    /// Allocation-free declaration used for this execution.
    pub fn plan(&self) -> Plan {
        self.plan
    }
    /// Last committed physical bits; callers cannot replace the state with a reference field.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }
    /// Complete attempted outcomes and accepted balance history.
    pub fn history(&self) -> &RunHistory {
        &self.history
    }
    /// Separate integration and observation charges for every recorded attempt.
    pub fn work(&self) -> &[AttemptWork] {
        &self.work
    }
    /// Accumulated independent observation charge, including failed admitted samples.
    pub fn observer_work(&self) -> BalanceObserverWork {
        self.observer.consumption()
    }
    /// Detailed current-attempt cache work for the opt-in cached profile.
    pub fn cache_work(&self) -> Option<AttemptCacheWork> {
        self.rhs.provider().cache_work()
    }
    /// Diagnostic lineage retained for the lifetime of this owner.
    pub fn origin(&self) -> Origin {
        self.origin
    }
}
