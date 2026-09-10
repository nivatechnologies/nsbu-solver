//! Actual accepted-state transactions feed independent off-stage reconstruction.
mod reconstruction_observer_support;
use nsbu_benchmarks::smooth_observer::reconstruction::ReconstructionObserver;
use nsbu_solver::{
    domain::TickClock,
    experiment::{control::Outcome, observer::BalanceObserver},
    integrators::method::Method,
    Complex64, SolverError,
};
use reconstruction_observer_support::Case;

fn probe() -> TickClock {
    TickClock::restore(-20, 1 << 20, 768, (1 << 20) - 768).unwrap()
}

#[test]
fn both_methods_supply_actual_nodes_and_independent_offstage_derivatives() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut case = Case::new(method, 5, 1e-2);
        assert!(case.observer.last_accepted_clocks().is_none());
        for _ in 0..2 {
            assert!(matches!(case.step(), Outcome::Committed(_)));
        }
        let clocks = case.observer.last_accepted_clocks().unwrap();
        assert_eq!(clocks.map(|clock| clock.elapsed()), [0, 1024, 2048]);
        let layout = case.state.plan().domain().layout();
        let mut value = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        let mut derivative = value.clone();
        for (axis, frequency) in [13.0_f64, 17.0, 19.0].into_iter().enumerate() {
            case.observer
                .reconstruct(probe(), axis, &mut value, &mut derivative)
                .unwrap();
            let mut mode = [0; 3];
            mode[(axis + 1) % 3] = 1;
            let index = layout.locate(mode).unwrap().0;
            let t = 768.0 / (1 << 20) as f64;
            let exact = Complex64::new(0.0, -(frequency * t).sin() / 2.0);
            let exact_derivative = Complex64::new(0.0, -frequency * (frequency * t).cos() / 2.0);
            assert!((value[index] - exact).norm_sqr().sqrt() < 1e-8);
            assert!((derivative[index] - exact_derivative).norm_sqr().sqrt() < 1e-6);
        }
        assert_eq!(case.observer.consumption().samples, 3);
        assert_eq!(case.observer.modal_visits(), 9 * layout.half_len());
        assert_eq!(case.observer.remaining_samples(), 2);
        assert_eq!(case.observer.limits_declared().samples, 5);
    }
}

#[test]
fn rejected_and_failed_proposals_preserve_the_accepted_ring() {
    let mut rejected = Case::new(Method::CoxMatthews, 5, 1e-30);
    let work = rejected.observer.consumption();
    assert!(matches!(rejected.step(), Outcome::Rejected(_)));
    assert!(rejected.observer.last_accepted_clocks().is_none());
    assert_eq!(rejected.observer.consumption(), work);
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut case = Case::new(method, 3, 1e-2);
        case.step();
        case.step();
        let clocks = case.observer.last_accepted_clocks();
        let mut before =
            vec![Complex64::new(0.0, 0.0); case.state.plan().domain().layout().half_len()];
        let mut derivative = before.clone();
        case.observer
            .reconstruct(probe(), 0, &mut before, &mut derivative)
            .unwrap();
        assert!(matches!(
            case.step(),
            Outcome::Refused {
                cause: SolverError::ProviderBudgetExceeded,
                ..
            }
        ));
        assert_eq!(case.state.clock().elapsed(), 2048);
        assert_eq!(case.observer.last_accepted_clocks(), clocks);
        let mut after = before.clone();
        case.observer
            .reconstruct(probe(), 0, &mut after, &mut derivative)
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(case.observer.remaining_samples(), 0);
    }
}

#[test]
fn constructor_and_stage_boundaries_reject_invalid_history() {
    let mut case = Case::new(Method::CoxMatthews, 5, 1e-2);
    assert_eq!(
        case.observer.measure(&case.state),
        Err(SolverError::StaleAttempt)
    );
    assert!(ReconstructionObserver::limits(case.state.plan().domain(), 0).is_err());
    assert!(matches!(case.step(), Outcome::Committed(_)));
    assert!(matches!(
        ReconstructionObserver::new(case.state.plan(), 5, &case.state),
        Err(SolverError::InvalidPayload)
    ));
    let mut output = vec![Complex64::new(0.0, 0.0); case.state.plan().domain().layout().half_len()];
    let mut derivative = output.clone();
    assert!(case
        .observer
        .reconstruct(probe(), 0, &mut output, &mut derivative)
        .is_err());
    assert!(case
        .observer
        .reconstruct(probe(), 3, &mut output, &mut derivative)
        .is_err());
}
