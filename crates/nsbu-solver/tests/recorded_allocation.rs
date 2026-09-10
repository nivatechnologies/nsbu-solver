//! Dedicated process checks complete recorded-step storage and allocation-free history commits.
mod mean_balance;
mod mean_source;
mod source_contract;
use nsbu_solver::diagnostics::balances::BalanceSample;
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::experiment::{
    control::{Configuration, Outcome},
    log::RunHistory,
    observer::{BalanceObserver, ObserverBounds},
    runner::recorded_step,
};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace, indicator::Tolerances, method::Method, trajectory::RunLimits,
    transaction::CandidateState,
};
use nsbu_solver::SolverError;
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

struct MeanObserver;
impl BalanceObserver for MeanObserver {
    fn bounds(&self) -> Option<ObserverBounds> {
        Some(ObserverBounds {
            storage_bytes: 0,
            work_units: 1,
        })
    }
    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError> {
        Ok(mean_balance::measured(state))
    }
}

fn main() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        probe(method);
    }
}
fn probe(method: Method) {
    let config = Configuration {
        limits: RunLimits {
            endpoint: 80,
            step_ticks: 4,
            maximum_attempts: 20,
        },
        method,
        tolerances: Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        },
    };
    let history_bytes = RunHistory::reservation(config).unwrap();
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostics = AttemptWorkspace::reservation_with_method(domain, method).unwrap();
    let overhead = history_bytes + 4096;
    let extra = ExtraStorage {
        fft: 0,
        force: 0,
        diagnostics,
        overhead,
    };
    let plan = ResourcePlan::new(domain, extra, 1 << 20, Epoch(0)).unwrap();
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let planning = Region::new(GLOBAL);
    let mut history = RunHistory::new(clock, config, history_bytes).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
    let mut source = mean_source::source(usize::MAX);
    assert!(planning.change().bytes_allocated <= plan.total());
    let region = Region::new(GLOBAL);
    for count in 1..=20 {
        let outcome = recorded_step(
            &mut state,
            &mut candidate,
            &mut workspace,
            &mut source,
            &mut MeanObserver,
            &mut history,
        )
        .unwrap();
        assert!(matches!(outcome, Outcome::Committed(_)));
        assert_eq!(history.records().len(), count);
        assert_eq!(history.controller().attempted(), count);
        assert_eq!(history.balance().samples(), count + 1);
    }
    assert_eq!(history.balance().clock(), state.clock());
    assert_eq!(history.controller().clock(), state.clock());
    assert_eq!(region.change(), Stats::default());
}
