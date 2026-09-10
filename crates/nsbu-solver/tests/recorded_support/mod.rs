//! Shared owned fixtures for actual recorded integration and refusal checks.
use super::{mean_balance, source_contract};
use nsbu_solver::{
    diagnostics::balances::BalanceSample,
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        observer::{BalanceObserver, ObserverBounds},
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace,
        transaction::{commit_candidate, CandidateState},
    },
    SolverError,
};

pub use super::mean_source::source;
pub use super::recorded_config::configuration;

pub struct Observer {
    pub bounds: Option<ObserverBounds>,
    pub calls: usize,
    pub last_clock: Option<TickClock>,
    pub fail_at: usize,
    pub invalid_at: usize,
}
impl Default for Observer {
    fn default() -> Self {
        Self {
            bounds: Some(ObserverBounds {
                storage_bytes: std::mem::size_of::<Self>(),
                work_units: 1,
            }),
            calls: 0,
            last_clock: None,
            fail_at: usize::MAX,
            invalid_at: usize::MAX,
        }
    }
}
impl BalanceObserver for Observer {
    fn bounds(&self) -> Option<ObserverBounds> {
        self.bounds
    }
    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError> {
        self.calls += 1;
        self.last_clock = Some(state.clock());
        if self.calls == self.fail_at {
            return Err(SolverError::ResourceLimit);
        }
        let mut sample = mean_balance::measured(state);
        if self.calls == self.invalid_at {
            sample.forcing_work = f64::NAN;
        }
        Ok(sample)
    }
}
pub struct Fixture {
    pub state: SpectralState,
    pub candidate: CandidateState,
    pub workspace: AttemptWorkspace,
    pub history: RunHistory,
}
impl Fixture {
    pub fn new(config: Configuration) -> Self {
        Self::with_overhead(config, 4096)
    }
    pub fn with_overhead(config: Configuration, overhead: usize) -> Self {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let scratch = AttemptWorkspace::reservation_with_method(domain, config.method).unwrap();
        let plan = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force: 0,
                diagnostics: scratch + std::mem::size_of::<Observer>(),
                overhead,
            },
            1 << 20,
            Epoch(0),
        )
        .unwrap();
        let clock = TickClock::from_rest(-12, 100).unwrap();
        let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
        let history =
            RunHistory::new(clock, config, RunHistory::reservation(config).unwrap()).unwrap();
        Self {
            state,
            candidate: CandidateState::new(plan, clock, Epoch(0)).unwrap(),
            workspace: AttemptWorkspace::new_with_method(plan, config.method).unwrap(),
            history,
        }
    }
    pub fn step(
        &mut self,
        source: &mut source_contract::Source,
        observer: &mut Observer,
    ) -> Result<Outcome, SolverError> {
        recorded_step(
            &mut self.state,
            &mut self.candidate,
            &mut self.workspace,
            source,
            observer,
            &mut self.history,
        )
    }
    pub fn bare(&mut self, ticks: u128) {
        let result = self
            .workspace
            .try_advance(
                &self.state,
                &mut self.candidate,
                ticks,
                configuration().tolerances,
                &mut source(usize::MAX),
            )
            .unwrap();
        commit_candidate(
            self.state.plan(),
            &mut self.state,
            &mut self.candidate,
            result.accepted.unwrap(),
        )
        .unwrap();
    }
}
