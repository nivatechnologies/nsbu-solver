use super::*;
use nsbu_solver::{
    domain::ExtraStorage,
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace, forcing::PrescribedForce, indicator::Tolerances, method::Method,
        rhs::SpectralRhs, trajectory::RunLimits, transaction::CandidateState,
    },
};

fn plan(n: usize, samples: usize, complete: bool) -> ResourcePlan {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0).unwrap();
    let limits = ReconstructionObserver::limits(domain, samples).unwrap();
    let configuration = configuration();
    let (force, attempts, history) = if complete {
        let provider = CyclicSine::new(domain).unwrap();
        (
            SpectralRhs::<CyclicSine>::reservation(domain, provider.limits().unwrap()).unwrap(),
            AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews).unwrap(),
            RunHistory::reservation(configuration).unwrap(),
        )
    } else {
        (0, 0, 0)
    };
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force,
            diagnostics: limits.storage_bytes + attempts,
            overhead: history,
        },
        8 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap()
}

fn configuration() -> Configuration {
    Configuration {
        method: Method::CoxMatthews,
        limits: RunLimits {
            endpoint: 4096,
            step_ticks: 1024,
            maximum_attempts: 4,
        },
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
    }
}

fn case(n: usize) -> (ReconstructionObserver, SpectralState) {
    let plan = plan(n, 4, false);
    let state =
        SpectralState::from_rest(plan, TickClock::from_rest(-20, 1 << 20).unwrap(), Epoch(0))
            .unwrap();
    let observer = ReconstructionObserver::new(plan, 4, &state).unwrap();
    (observer, state)
}

fn advanced_case() -> (ReconstructionObserver, SpectralState) {
    let plan = plan(4, 4, true);
    let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
    let configuration = configuration();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(plan, Method::CoxMatthews).unwrap();
    let provider = CyclicSine::new(plan.domain()).unwrap();
    let reservation =
        SpectralRhs::<CyclicSine>::reservation(plan.domain(), provider.limits().unwrap()).unwrap();
    let mut rhs = SpectralRhs::new(plan.domain(), provider, 0.3, reservation).unwrap();
    let mut observer = ReconstructionObserver::new(plan, 4, &state).unwrap();
    let mut history = RunHistory::new(
        clock,
        configuration,
        RunHistory::reservation(configuration).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        recorded_step(
            &mut state,
            &mut candidate,
            &mut workspace,
            &mut rhs,
            &mut observer,
            &mut history,
        )
        .unwrap(),
        Outcome::Committed(_)
    ));
    (observer, state)
}

#[test]
fn private_ring_and_budget_guards_cover_both_transaction_paths() {
    let (mut observer, state) = case(4);
    observer.modal_visits = observer.limits.modal_visits;
    assert_eq!(
        observer.charge_modal(),
        Err(SolverError::ProviderBudgetExceeded)
    );
    observer.modal_visits = 0;

    observer.has_pending = true;
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::StaleAttempt)
    );
    observer.has_pending = false;
    let (_, foreign) = case(8);
    assert_eq!(
        observer.admit_endpoint(&foreign),
        Err(SolverError::StaleAttempt)
    );

    BalanceObserverContract::commit_pending(&mut observer);
    assert_eq!(observer.accepted_count, 1);
    observer.has_pending = true;
    BalanceObserverContract::commit_pending(&mut observer);
    assert_eq!(observer.accepted_count, 2);
    observer.has_pending = true;
    BalanceObserverContract::commit_pending(&mut observer);
    assert_eq!(observer.accepted_count, 3);
    observer.has_pending = true;
    BalanceObserverContract::commit_pending(&mut observer);
    assert_eq!(observer.accepted_count, 3);
}

#[test]
fn private_endpoint_admission_distinguishes_lineage_and_clock_failures() {
    let (mut observer, state) = advanced_case();
    assert_eq!(observer.accepted_count, 2);

    observer.accepted[1].epoch = state.epoch();
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::StaleAttempt)
    );
    observer.accepted[1].epoch = Epoch(0);
    observer.accepted[1].steps = state.accepted_steps();
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::StaleAttempt)
    );
    observer.accepted[1].steps = 0;

    observer.accepted[1].clock = None;
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::InvalidClock)
    );
    observer.accepted[1].clock =
        Some(TickClock::restore(-19, 1 << 19, 512, (1 << 19) - 512).unwrap());
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::InvalidClock)
    );
    observer.accepted[1].clock =
        Some(TickClock::restore(-20, (1 << 20) - 1, 512, (1 << 20) - 513).unwrap());
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::InvalidClock)
    );
    observer.accepted[1].clock = Some(state.clock());
    assert_eq!(
        observer.admit_endpoint(&state),
        Err(SolverError::InvalidClock)
    );
}
