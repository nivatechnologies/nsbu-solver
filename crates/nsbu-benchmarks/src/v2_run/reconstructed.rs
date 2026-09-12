//! Exact-v2 runtime owner with transactional accepted-node reconstruction.

use super::{plan, work, AttemptWork, Origin, Settings};
use crate::{
    runtime_force::{AttemptCacheWork, IntegrationMode, RunForce},
    smooth_observer::{v2_reconstruction::V2ReconstructionObserver, BalanceObserverWork},
};
use nsbu_solver::{
    domain::{Epoch, ExtraStorage, ResourcePlan, SpectralState},
    experiment::{control::Outcome, log::RunHistory, runner::recorded_step},
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs, transaction::CandidateState},
    SolverError,
};

/// Complete exact-v2 reconstruction admission, including the initial rest sample.
#[derive(Debug, Clone, Copy)]
pub struct ReconstructedPlan {
    settings: Settings,
    resources: ResourcePlan,
    observer_samples: usize,
    integration: [usize; 3],
}

impl ReconstructedPlan {
    /// Admit integration plus independently evaluated accepted-node RHS history.
    pub fn from_rest(settings: Settings, cap: usize) -> Result<Self, SolverError> {
        Self::admit(settings, IntegrationMode::Direct, cap)
    }

    pub(crate) fn from_run_plan(plan: super::Plan, cap: usize) -> Result<Self, SolverError> {
        Self::admit(plan.settings(), plan.integration_mode(), cap)
    }

    fn admit(settings: Settings, mode: IntegrationMode, cap: usize) -> Result<Self, SolverError> {
        if std::mem::size_of::<ReconstructedPlan>() != 480
            || std::mem::size_of::<ReconstructedRun>() != 7136
        {
            return Err(SolverError::ResourceLimit);
        }
        plan::validate_settings(settings)?;
        let force_limits = settings.force.integration_limits(settings.domain, mode)?;
        plan::validate_final_step(settings, force_limits.remaining_divisor)?;
        let attempts = settings.configuration.limits.maximum_attempts;
        let observer_samples = attempts.checked_add(1).ok_or(SolverError::SizeOverflow)?;
        let calls = settings
            .configuration
            .method
            .rhs_calls()
            .checked_mul(attempts)
            .ok_or(SolverError::SizeOverflow)?;
        let integration = [
            calls,
            force_limits
                .work_units
                .checked_mul(calls)
                .ok_or(SolverError::SizeOverflow)?,
            force_limits
                .scalar_transforms
                .checked_add(10)
                .and_then(|n| n.checked_mul(calls))
                .ok_or(SolverError::SizeOverflow)?,
        ];
        let observer =
            V2ReconstructionObserver::v2_limits(settings.domain, settings.force, observer_samples)?;
        let force = SpectralRhs::<RunForce>::reservation(settings.domain, force_limits)?;
        let diagnostics = AttemptWorkspace::reservation_with_method(
            settings.domain,
            settings.configuration.method,
        )?
        .checked_add(observer.storage_bytes)
        .ok_or(SolverError::SizeOverflow)?;
        let overhead = RunHistory::reservation(settings.configuration)?
            .checked_add(work::reservation(attempts)?)
            .and_then(|n| n.checked_add(std::mem::size_of::<ReconstructedRun>()))
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let resources = ResourcePlan::new(
            settings.domain,
            ExtraStorage {
                fft: 0,
                force,
                diagnostics,
                overhead,
            },
            cap,
            Epoch(0),
        )?;
        Ok(Self {
            settings,
            resources,
            observer_samples,
            integration,
        })
    }

    /// Immutable exact-v2 settings.
    pub fn settings(self) -> Settings {
        self.settings
    }
    /// Complete admitted numerical resources.
    pub fn resources(self) -> ResourcePlan {
        self.resources
    }
    /// Initial rest plus every possible accepted endpoint evaluation.
    pub fn observer_samples(self) -> usize {
        self.observer_samples
    }
    /// Maximum integration calls, provider work units and scalar transforms.
    pub fn integration_limits(self) -> [usize; 3] {
        self.integration
    }

    /// Integration-only force policy inherited from the ordinary branch plan.
    pub fn integration_mode(self) -> IntegrationMode {
        plan::direct_or_cached(self.settings, self.resources)
    }
}

/// From-rest exact-v2 owner retaining three committed endpoint value/RHS nodes.
pub struct ReconstructedRun {
    plan: ReconstructedPlan,
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<RunForce>,
    observer: V2ReconstructionObserver,
    history: RunHistory,
    work: Vec<AttemptWork>,
}

impl ReconstructedRun {
    /// Construct a private from-rest owner; no external reconstruction import is accepted.
    pub fn from_rest(plan: ReconstructedPlan) -> Result<Self, SolverError> {
        let settings = plan.settings;
        let resources = plan.resources;
        let state = SpectralState::from_rest(resources, settings.initial_clock, Epoch(0))?;
        let candidate = CandidateState::new(resources, settings.initial_clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(resources, settings.configuration.method)?;
        let force_limits = settings
            .force
            .integration_limits(settings.domain, plan.integration_mode())?;
        let force = settings.force.build_integration(
            settings.domain,
            plan.integration_mode(),
            force_limits.storage_bytes,
        )?;
        let rhs = SpectralRhs::new(
            settings.domain,
            force,
            settings.advective_limit,
            resources.classes()[5],
        )?;
        let observer = V2ReconstructionObserver::new_v2(
            resources,
            settings.force,
            plan.observer_samples,
            &state,
        )?;
        let history = RunHistory::new(
            settings.initial_clock,
            settings.configuration,
            RunHistory::reservation(settings.configuration)?,
        )?;
        let mut work = Vec::new();
        work.try_reserve_exact(settings.configuration.limits.maximum_attempts)
            .map_err(|_| SolverError::AllocationFailed)?;
        Ok(Self {
            plan,
            state,
            candidate,
            attempts,
            rhs,
            observer,
            history,
            work,
        })
    }

    /// Attempt one interval; rejected/refused proposals cannot enter the accepted-node ring.
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
        self.work.push(AttemptWork {
            integration,
            observation: subtract(after, before),
        });
        Ok(outcome)
    }

    /// Immutable reconstruction admission.
    pub fn plan(&self) -> ReconstructedPlan {
        self.plan
    }
    /// Last committed physical state.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }
    /// Accepted controller and balance history.
    pub fn history(&self) -> &RunHistory {
        &self.history
    }
    /// Per-attempt integration and endpoint-observation charges.
    pub fn work(&self) -> &[AttemptWork] {
        &self.work
    }
    /// Includes the separately evaluated initial rest node.
    pub fn observer_work(&self) -> BalanceObserverWork {
        self.observer.consumption()
    }
    /// Detailed current-attempt cache work for the opt-in cached profile.
    pub fn cache_work(&self) -> Option<AttemptCacheWork> {
        self.rhs.provider().cache_work()
    }
    /// Accepted-node reconstruction access; scratch remains observer-owned.
    pub fn observer(&self) -> &V2ReconstructionObserver {
        &self.observer
    }
    /// This owner can only be constructed internally from exact rest.
    pub fn origin(&self) -> Origin {
        Origin::InternalFromRest
    }
}

fn subtract(after: BalanceObserverWork, before: BalanceObserverWork) -> BalanceObserverWork {
    BalanceObserverWork {
        samples: after.samples - before.samples,
        work_units: after.work_units - before.work_units,
        scalar_transforms: after.scalar_transforms - before.scalar_transforms,
    }
}
