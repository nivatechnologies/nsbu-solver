//! Owned, restartable fixed-step runs for the immutable [`super::smooth::CyclicSine`] profile.
use crate::{
    smooth::CyclicSine,
    smooth_observer::{BalanceObserver, BalanceObserverWork},
};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Controller, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace, forcing::PrescribedForce, rhs::SpectralRhs,
        transaction::CandidateState,
    },
    lineage::PhysicalImage,
    SolverError,
};

pub mod archive;

/// Diagnostic origin of an owned smooth-run payload; neither value is a PDE qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Constructed by this process from an admitted zero state.
    InternalFromRest,
    /// Decoded from external bytes whose provenance was not reconstructed.
    ExternalUnverified,
}

/// Integration work retained for one attempted interval, including a recorded refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationWork {
    calls: usize,
    work_units: usize,
    scalar_transforms: usize,
}

impl IntegrationWork {
    /// RHS calls made during the attempt.
    pub fn calls(self) -> usize {
        self.calls
    }
    /// Charged provider work units during the attempt.
    pub fn work_units(self) -> usize {
        self.work_units
    }
    /// Charged provider and operator scalar transforms during the attempt.
    pub fn scalar_transforms(self) -> usize {
        self.scalar_transforms
    }
}

/// Trusted in-memory snapshot of one owned `CyclicSine` run.
///
/// It is intentionally constructed only by [`SmoothRun::snapshot`] and consumed only by
/// [`SmoothRun::restore`], so callers cannot combine an unrelated state and history.
#[derive(Debug)]
pub struct SmoothSnapshot {
    physical: PhysicalImage,
    history: RunHistory,
    work: Vec<IntegrationWork>,
    observer_work: BalanceObserverWork,
    configuration: Configuration,
    initial_clock: TickClock,
    observer_samples: usize,
    advective_limit: f64,
    origin: Origin,
}

/// An owned fixed-step execution with fresh, private numerical and diagnostic scratch.
///
/// This profile supports only `CyclicSine`, whose provider has immutable mathematical state.
/// Reconstruction is inactive; this object carries no external artifacts or PDE qualification.
pub struct SmoothRun {
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<CyclicSine>,
    observer: BalanceObserver,
    history: RunHistory,
    work: Vec<IntegrationWork>,
    configuration: Configuration,
    initial_clock: TickClock,
    observer_samples: usize,
    advective_limit: f64,
    origin: Origin,
}

const ALLOCATOR_ALLOWANCE: usize = 4096;

/// Allocation-free admission evidence for a bounded [`SmoothRun`] from rest.
#[derive(Debug, Clone, Copy)]
pub struct SmoothPlan {
    plan: ResourcePlan,
    configuration: Configuration,
    observer_samples: usize,
    integration_calls: usize,
    integration_work_units: usize,
    integration_scalar_transforms: usize,
}

impl SmoothPlan {
    /// Validate the same complete configuration and resource ledger used by construction.
    pub fn from_rest(
        domain: Domain,
        clock: TickClock,
        configuration: Configuration,
        observer_samples: usize,
        advective_limit: f64,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let rest = TickClock::from_rest(clock.exponent(), clock.target())?;
        if clock != rest {
            return Err(SolverError::InvalidClock);
        }
        let controller = Controller::new(clock, configuration)?;
        if !advective_limit.is_finite() || advective_limit <= 0.0 {
            return Err(SolverError::InvalidStep);
        }
        let source = CyclicSine::new(domain)?;
        let limits = source.limits().ok_or(SolverError::UnknownProviderCost)?;
        let integration_calls = configuration
            .method
            .rhs_calls()
            .checked_mul(controller.required_commits())
            .ok_or(SolverError::SizeOverflow)?;
        let integration_work_units = limits
            .work_units
            .checked_mul(integration_calls)
            .ok_or(SolverError::SizeOverflow)?;
        let integration_scalar_transforms = limits
            .scalar_transforms
            .checked_add(10)
            .and_then(|value| value.checked_mul(integration_calls))
            .ok_or(SolverError::SizeOverflow)?;
        Ok(Self {
            plan: plan(domain, configuration, observer_samples, cap)?,
            configuration,
            observer_samples,
            integration_calls,
            integration_work_units,
            integration_scalar_transforms,
        })
    }

    /// Checked resource ledger without numerical-state allocation.
    pub fn resources(self) -> ResourcePlan {
        self.plan
    }
    /// Frozen run policy admitted by this plan.
    pub fn configuration(self) -> Configuration {
        self.configuration
    }
    /// Number of independently measured accepted states admitted by the plan.
    pub fn observer_samples(self) -> usize {
        self.observer_samples
    }
    /// Declared integration RHS calls at the endpoint.
    pub fn integration_calls(self) -> usize {
        self.integration_calls
    }
    /// Declared integration provider work units at the endpoint.
    pub fn integration_work_units(self) -> usize {
        self.integration_work_units
    }
    /// Declared integration scalar transforms at the endpoint.
    pub fn integration_scalar_transforms(self) -> usize {
        self.integration_scalar_transforms
    }
}

impl SmoothRun {
    /// Build a finite from-rest run after preflighting force, diagnostics, history and ledger.
    pub fn from_rest(
        domain: Domain,
        clock: TickClock,
        configuration: Configuration,
        observer_samples: usize,
        advective_limit: f64,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let admission = SmoothPlan::from_rest(
            domain,
            clock,
            configuration,
            observer_samples,
            advective_limit,
            cap,
        )?;
        let plan = admission.resources();
        let source = CyclicSine::new(domain)?;
        let force_cap = plan.classes()[5];
        let history_cap = RunHistory::reservation(configuration)?;
        let state = SpectralState::from_rest(plan, clock, Epoch(0))?;
        let candidate = CandidateState::new(plan, clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(plan, configuration.method)?;
        let rhs = SpectralRhs::new(domain, source, advective_limit, force_cap)?;
        let observer = BalanceObserver::new(plan, observer_samples)?;
        let history = RunHistory::new(clock, configuration, history_cap)?;
        let work = work_storage(configuration.limits.maximum_attempts)?;
        Ok(Self {
            state,
            candidate,
            attempts,
            rhs,
            observer,
            history,
            work,
            configuration,
            initial_clock: clock,
            observer_samples,
            advective_limit,
            origin: Origin::InternalFromRest,
        })
    }

    /// Perform one recorded attempt, retaining a zero charge when RHS setup was never reached.
    pub fn step(&mut self) -> Result<Outcome, SolverError> {
        let generation = self.rhs.consumption_epoch();
        let outcome = recorded_step(
            &mut self.state,
            &mut self.candidate,
            &mut self.attempts,
            &mut self.rhs,
            &mut self.observer,
            &mut self.history,
        )?;
        let work = if self.rhs.consumption_epoch() == generation {
            IntegrationWork {
                calls: 0,
                work_units: 0,
                scalar_transforms: 0,
            }
        } else {
            let [calls, work_units, scalar_transforms] = self.rhs.consumption();
            IntegrationWork {
                calls,
                work_units,
                scalar_transforms,
            }
        };
        self.work.push(work);
        Ok(outcome)
    }

    /// Capture physical bits, replayed raw history, integration charges and observer charges.
    pub fn snapshot(&self, cap: usize) -> Result<SmoothSnapshot, SolverError> {
        if self.snapshot_reservation()? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let physical = PhysicalImage::capture(&self.state, cap)?;
        let history = RunHistory::replay(
            self.initial_clock,
            self.configuration,
            self.history.records(),
            RunHistory::reservation(self.configuration)?,
        )?;
        let mut work = work_storage(self.configuration.limits.maximum_attempts)?;
        work.extend_from_slice(&self.work);
        Ok(SmoothSnapshot {
            physical,
            history,
            work,
            observer_work: self.observer.consumption(),
            configuration: self.configuration,
            initial_clock: self.initial_clock,
            observer_samples: self.observer_samples,
            advective_limit: self.advective_limit,
            origin: self.origin,
        })
    }

    /// Complete owned snapshot allocation, excluding caller allocator overhead.
    pub fn snapshot_reservation(&self) -> Result<usize, SolverError> {
        let ledger = ledger_bytes(self.configuration.limits.maximum_attempts)?;
        PhysicalImage::reservation(self.state.plan())?
            .checked_add(RunHistory::reservation(self.configuration)?)
            .and_then(|bytes| bytes.checked_add(ledger))
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<SmoothSnapshot>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Rebuild fresh scratch from a trusted snapshot under the same fixed configuration.
    pub fn restore(
        snapshot: SmoothSnapshot,
        configuration: Configuration,
        cap: usize,
    ) -> Result<Self, SolverError> {
        if !same_configuration(snapshot.configuration, configuration) {
            return Err(SolverError::InvalidPayload);
        }
        let plan = snapshot.physical.state().plan();
        if plan.total() > cap || snapshot.history.records().len() != snapshot.work.len() {
            return Err(SolverError::ResourceLimit);
        }
        let state = snapshot.physical.into_state();
        if state.clock() != snapshot.history.controller().clock()
            || state.accepted_steps() != snapshot.history.controller().committed() as u128
        {
            return Err(SolverError::InvalidPayload);
        }
        let domain = plan.domain();
        let source = CyclicSine::new(domain)?;
        let candidate = CandidateState::new(plan, snapshot.initial_clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(plan, configuration.method)?;
        let rhs = SpectralRhs::new(domain, source, snapshot.advective_limit, plan.classes()[5])?;
        let observer =
            BalanceObserver::restore(plan, snapshot.observer_samples, snapshot.observer_work)?;
        Ok(Self {
            state,
            candidate,
            attempts,
            rhs,
            observer,
            history: snapshot.history,
            work: snapshot.work,
            configuration,
            initial_clock: snapshot.initial_clock,
            observer_samples: snapshot.observer_samples,
            advective_limit: snapshot.advective_limit,
            origin: snapshot.origin,
        })
    }

    /// Current physical state, exposed read-only.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }
    /// Complete recorded controller and balance history, exposed read-only.
    pub fn history(&self) -> &RunHistory {
        &self.history
    }
    /// Per-attempt integration charges in the same order as history records.
    pub fn work(&self) -> &[IntegrationWork] {
        &self.work
    }
    /// Accumulated independent diagnostic charge.
    pub fn observer_work(&self) -> BalanceObserverWork {
        self.observer.consumption()
    }
    /// Diagnostic origin retained through snapshots and later external exports.
    pub fn origin(&self) -> Origin {
        self.origin
    }
}

fn plan(
    domain: Domain,
    configuration: Configuration,
    observer_samples: usize,
    cap: usize,
) -> Result<ResourcePlan, SolverError> {
    let source = CyclicSine::new(domain)?;
    let limits = source.limits().ok_or(SolverError::UnknownProviderCost)?;
    let force = SpectralRhs::<CyclicSine>::reservation(domain, limits)?;
    let diagnostics = AttemptWorkspace::reservation_with_method(domain, configuration.method)?
        .checked_add(BalanceObserver::limits(domain, observer_samples)?.storage_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    let overhead = RunHistory::reservation(configuration)?
        .checked_add(ledger_bytes(configuration.limits.maximum_attempts)?)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<SmoothRun>()))
        .and_then(|bytes| bytes.checked_add(ALLOCATOR_ALLOWANCE))
        .ok_or(SolverError::SizeOverflow)?;
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force,
            diagnostics,
            overhead,
        },
        cap,
        Epoch(0),
    )
}

fn ledger_bytes(attempts: usize) -> Result<usize, SolverError> {
    attempts
        .checked_mul(std::mem::size_of::<IntegrationWork>())
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Vec<IntegrationWork>>()))
        .ok_or(SolverError::SizeOverflow)
}

fn work_storage(attempts: usize) -> Result<Vec<IntegrationWork>, SolverError> {
    let mut work = Vec::new();
    work.try_reserve_exact(attempts)
        .map_err(|_| SolverError::AllocationFailed)?;
    Ok(work)
}

fn same_configuration(left: Configuration, right: Configuration) -> bool {
    left.method == right.method
        && left.limits.endpoint == right.limits.endpoint
        && left.limits.step_ticks == right.limits.step_ticks
        && left.limits.maximum_attempts == right.limits.maximum_attempts
        && left.tolerances.absolute.map(f64::to_bits) == right.tolerances.absolute.map(f64::to_bits)
        && left.tolerances.relative.map(f64::to_bits) == right.tolerances.relative.map(f64::to_bits)
}
