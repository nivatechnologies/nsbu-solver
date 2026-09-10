//! Complete owner restoration retains the accepted interpolant and next transaction.
mod owned_reconstruction_support;
use nsbu_benchmarks::smooth_run::{ReconstructedPlan, ReconstructedRun};
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Outcome,
    integrators::method::Method,
    SolverError,
};
use owned_reconstruction_support::{compare, configuration, run, CAP};

#[test]
fn every_ring_phase_restores_the_next_cm_and_ho_commit_bit_for_bit() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for steps in 0..4 {
            let mut original = run(method, 1e-2, 5);
            for _ in 0..steps {
                original.step().unwrap();
            }
            let snapshot = original
                .snapshot(original.snapshot_reservation().unwrap())
                .unwrap();
            let mut restored =
                ReconstructedRun::restore(snapshot, configuration(method, 1e-2), CAP).unwrap();
            compare(&original, &restored);
            assert_eq!(original.step(), restored.step());
            assert!(matches!(
                original.history().records().last().unwrap().outcome,
                Outcome::Committed(_)
            ));
            compare(&original, &restored);
        }
    }
}

#[test]
fn restoration_retains_rejection_and_observer_exhaustion_without_budget_reset() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method, 1e-30, 5);
        let mut restored = ReconstructedRun::restore(
            original.snapshot(CAP).unwrap(),
            configuration(method, 1e-30),
            CAP,
        )
        .unwrap();
        assert_eq!(original.step(), restored.step());
        assert!(matches!(
            original.history().records()[0].outcome,
            Outcome::Rejected(_)
        ));
        compare(&original, &restored);
        let mut original = run(method, 1e-2, 3);
        original.step().unwrap();
        original.step().unwrap();
        let mut restored = ReconstructedRun::restore(
            original.snapshot(CAP).unwrap(),
            configuration(method, 1e-2),
            CAP,
        )
        .unwrap();
        assert_eq!(original.step(), restored.step());
        assert!(matches!(
            original.history().records().last().unwrap().outcome,
            Outcome::Refused {
                cause: SolverError::ProviderBudgetExceeded,
                ..
            }
        ));
        compare(&original, &restored);
        let terminal = ReconstructedRun::restore(
            original.snapshot(CAP).unwrap(),
            configuration(method, 1e-2),
            CAP,
        )
        .unwrap();
        compare(&original, &terminal);
    }
}

#[test]
fn reconstruction_owner_preflights_live_and_snapshot_storage() {
    let original = run(Method::CoxMatthews, 1e-2, 5);
    let plan = ReconstructedPlan::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-20, 1 << 20).unwrap(),
        configuration(Method::CoxMatthews, 1e-2),
        5,
        0.3,
        CAP,
    )
    .unwrap();
    assert_eq!(plan.resources(), original.state().plan());
    let snapshot = original.snapshot_reservation().unwrap();
    assert_eq!(
        original.snapshot(snapshot - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    assert!(matches!(
        ReconstructedRun::restore(
            original.snapshot(snapshot).unwrap(),
            configuration(Method::CoxMatthews, 1e-2),
            1
        ),
        Err(SolverError::ResourceLimit)
    ));
}
