//! Complete owner restoration retains the accepted interpolant and next transaction.
mod owned_reconstruction_support;
use nsbu_benchmarks::smooth_run::{ReconstructedPlan, ReconstructedRun};
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Outcome,
    integrators::method::Method,
    Complex64, SolverError,
};
use owned_reconstruction_support::{configuration, run, CAP};

fn compare(left: &ReconstructedRun, right: &ReconstructedRun) {
    assert_eq!(left.state().clock(), right.state().clock());
    assert_eq!(left.state().epoch(), right.state().epoch());
    assert_eq!(left.work(), right.work());
    assert_eq!(left.observer_work(), right.observer_work());
    assert_eq!(
        left.observer().modal_visits(),
        right.observer().modal_visits()
    );
    assert_eq!(
        left.observer().remaining_samples(),
        right.observer().remaining_samples()
    );
    assert_eq!(
        left.history().controller().stopped(),
        right.history().controller().stopped()
    );
    assert_eq!(
        left.history().records().len(),
        right.history().records().len()
    );
    for (a, b) in left
        .history()
        .records()
        .iter()
        .zip(right.history().records())
    {
        assert_eq!(
            (a.start, a.outcome, a.sample),
            (b.start, b.outcome, b.sample)
        );
    }
    for axis in 0..3 {
        assert_bits(
            left.state().component(axis).unwrap(),
            right.state().component(axis).unwrap(),
        );
    }
    compare_reconstruction(left, right);
}
fn assert_bits(left: &[Complex64], right: &[Complex64]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(
            [a.re.to_bits(), a.im.to_bits()],
            [b.re.to_bits(), b.im.to_bits()]
        );
    }
}
fn compare_reconstruction(left: &ReconstructedRun, right: &ReconstructedRun) {
    let clocks = left.observer().last_accepted_clocks();
    assert_eq!(clocks, right.observer().last_accepted_clocks());
    let Some(clocks) = clocks else {
        return;
    };
    let elapsed = clocks[1].elapsed() + 1;
    let probe = TickClock::restore(
        clocks[0].exponent(),
        clocks[0].target(),
        elapsed,
        clocks[0].target() - elapsed,
    )
    .unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); left.state().plan().domain().layout().half_len()];
    let [mut a, mut b, mut da, mut db] = [zero.clone(), zero.clone(), zero.clone(), zero];
    for axis in 0..3 {
        left.observer()
            .reconstruct(probe, axis, &mut a, &mut da)
            .unwrap();
        right
            .observer()
            .reconstruct(probe, axis, &mut b, &mut db)
            .unwrap();
        assert_bits(&a, &b);
        assert_bits(&da, &db);
    }
}

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
