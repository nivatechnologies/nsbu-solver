//! Owned, restartable fixed-step runs for the immutable [`super::smooth::CyclicSine`] profile.
use crate::{
    smooth::CyclicSine,
    smooth_observer::{BalanceObserver, BalanceObserverWork},
};
use nsbu_solver::{
    domain::{Domain, Epoch, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs, transaction::CandidateState},
    lineage::PhysicalImage,
    SolverError,
};

pub mod archive;
pub mod observation;
mod plan;
pub mod reconstructed_archive;
pub mod replay;
use observation::Observation;
pub use plan::OwnedPlan;

/// Owned balance-only CyclicSine run; version-one archives use this profile.
pub type SmoothRun = OwnedRun<BalanceObserver>;
/// Trusted balance-only snapshot.
pub type SmoothSnapshot = OwnedSnapshot<BalanceObserver>;
/// Allocation-free balance-only admission plan.
pub type SmoothPlan = OwnedPlan<BalanceObserver>;
/// Owned CyclicSine run retaining independent accepted-node reconstruction.
pub type ReconstructedRun =
    OwnedRun<crate::smooth_observer::reconstruction::ReconstructionObserver>;
/// Trusted snapshot including accepted reconstruction and spent observation work.
pub type ReconstructedSnapshot =
    OwnedSnapshot<crate::smooth_observer::reconstruction::ReconstructionObserver>;
/// Allocation-free admission for the reconstruction-enabled smooth profile.
pub type ReconstructedPlan =
    OwnedPlan<crate::smooth_observer::reconstruction::ReconstructionObserver>;

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
pub struct OwnedSnapshot<O: Observation> {
    physical: PhysicalImage,
    history: RunHistory,
    work: Vec<IntegrationWork>,
    observer_snapshot: O::Snapshot,
    configuration: Configuration,
    initial_clock: TickClock,
    observer_samples: usize,
    advective_limit: f64,
    origin: Origin,
}

/// An owned fixed-step execution with fresh, private numerical and diagnostic scratch.
///
/// This profile supports only `CyclicSine`, whose provider has immutable mathematical state.
/// Its observation profile determines the accepted diagnostics retained in snapshots.
/// This object carries no external artifacts or PDE qualification.
pub struct OwnedRun<O: Observation> {
    state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<CyclicSine>,
    observer: O,
    history: RunHistory,
    work: Vec<IntegrationWork>,
    configuration: Configuration,
    initial_clock: TickClock,
    observer_samples: usize,
    advective_limit: f64,
    origin: Origin,
}

impl<O: Observation> OwnedRun<O> {
    /// Build a finite from-rest run after preflighting force, diagnostics, history and ledger.
    pub fn from_rest(
        domain: Domain,
        clock: TickClock,
        configuration: Configuration,
        observer_samples: usize,
        advective_limit: f64,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let admission = OwnedPlan::<O>::from_rest(
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
        let observer = O::from_rest(plan, observer_samples, &state)?;
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
    pub fn snapshot(&self, cap: usize) -> Result<OwnedSnapshot<O>, SolverError> {
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
        Ok(OwnedSnapshot {
            physical,
            history,
            work,
            observer_snapshot: self.observer.capture(cap)?,
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
        let observation = O::snapshot_reservation(self.state.plan().domain())?;
        PhysicalImage::reservation(self.state.plan())?
            .checked_add(RunHistory::reservation(self.configuration)?)
            .and_then(|bytes| bytes.checked_add(observation))
            .and_then(|bytes| bytes.checked_add(ledger))
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<OwnedSnapshot<O>>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Rebuild fresh scratch from a trusted snapshot under the same fixed configuration.
    pub fn restore(
        snapshot: OwnedSnapshot<O>,
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
        let observer = O::restore(
            plan,
            snapshot.observer_samples,
            &state,
            snapshot.observer_snapshot,
        )?;
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
    /// Read-only accepted diagnostics. Its type reflects the selected observation profile.
    pub fn observer(&self) -> &O {
        &self.observer
    }
    /// Diagnostic origin retained through snapshots and later external exports.
    pub fn origin(&self) -> Origin {
        self.origin
    }
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
