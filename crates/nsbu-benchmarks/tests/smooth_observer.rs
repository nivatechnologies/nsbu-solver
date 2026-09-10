//! Production observer checks use accepted from-rest trajectories, never an RHS workspace.
mod smooth_observer_support;

use nsbu_benchmarks::smooth_observer::{BalanceObserver, BalanceObserverWork};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan},
    experiment::observer::BalanceObserver as BalanceObserverContract,
    integrators::method::Method,
    SolverError,
};
use smooth_observer_support::run_with_observer;

#[test]
fn from_rest_and_accepted_cm_and_ho_samples_are_independent() {
    let mut results = Vec::new();
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let (mut run, mut observer, limits) = run_with_observer(method, 2);
        let bounds = BalanceObserverContract::bounds(&observer).expect("per-sample bounds");
        assert_eq!(bounds.storage_bytes, limits.storage_bytes);
        assert_eq!(bounds.work_units, limits.work_units / limits.samples);
        let rest = observer.sample(&run.state).expect("rest sample");
        assert_eq!(
            rest,
            nsbu_solver::diagnostics::balances::BalanceSample::REST
        );
        run.advance(4096, 1024);
        let before: [Vec<_>; 3] =
            std::array::from_fn(|axis| run.state.component(axis).expect("fixed axis").to_vec());
        let sample = observer.sample(&run.state).expect("accepted sample");
        for (axis, expected) in before.iter().enumerate() {
            assert_eq!(run.state.component(axis).expect("fixed axis"), expected);
        }
        assert!(sample.energy.is_finite() && sample.energy > 0.0);
        assert!(sample.enstrophy.is_finite() && sample.enstrophy > 0.0);
        assert_eq!(observer.consumption().samples, 2);
        assert_eq!(observer.consumption().work_units, limits.work_units);
        assert_eq!(
            observer.consumption().scalar_transforms,
            limits.scalar_transforms
        );
        results.push(sample);
    }
    assert!((results[0].energy - results[1].energy).abs() < 2e-5);
    assert!((results[0].enstrophy - results[1].enstrophy).abs() < 2e-4);
}

#[test]
fn capacity_and_plan_reservations_refuse_before_observer_allocation() {
    let (run, mut observer, limits) = run_with_observer(Method::CoxMatthews, 1);
    let bounds = BalanceObserverContract::bounds(&observer).expect("declared observer bounds");
    assert_eq!(bounds.storage_bytes, limits.storage_bytes);
    assert_eq!(bounds.work_units, limits.work_units);
    BalanceObserverContract::measure(&mut observer, &run.state).expect("first bounded sample");
    assert_eq!(
        observer.sample(&run.state),
        Err(SolverError::ProviderBudgetExceeded)
    );

    let domain = Domain::new([4; 3], [1.0; 3], 1.0).expect("fixed smooth domain");
    let insufficient = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: limits.storage_bytes - 1,
            overhead: 0,
        },
        8 * 1024 * 1024,
        Epoch(0),
    )
    .expect("base plan permits an undersized diagnostic declaration");
    assert!(matches!(
        BalanceObserver::restore(insufficient, 1, BalanceObserverWork::default()),
        Err(SolverError::ResourceLimit)
    ));
    assert_eq!(
        BalanceObserver::limits(domain, 0),
        Err(SolverError::ResourceLimit)
    );
}

#[test]
fn restored_observer_matches_uninterrupted_measurement_and_rejects_bad_ledgers() {
    let (mut run, mut uninterrupted, limits) = run_with_observer(Method::CoxMatthews, 2);
    uninterrupted.sample(&run.state).expect("initial sample");
    let saved = uninterrupted.consumption();
    let mut restored = BalanceObserver::restore(run.state.plan(), 2, saved)
        .expect("exact spent ledger restores fresh scratch");
    run.advance(4096, 1024);
    assert_eq!(
        restored.sample(&run.state).expect("restored sample"),
        uninterrupted
            .sample(&run.state)
            .expect("uninterrupted sample")
    );
    assert_eq!(restored.consumption(), uninterrupted.consumption());
    assert_eq!(restored.consumption().work_units, limits.work_units);
    assert!(matches!(
        BalanceObserver::restore(
            run.state.plan(),
            2,
            BalanceObserverWork {
                samples: 1,
                work_units: 0,
                scalar_transforms: 0,
            },
        ),
        Err(SolverError::InvalidPayload)
    ));
    assert!(matches!(
        BalanceObserver::restore(
            run.state.plan(),
            2,
            BalanceObserverWork {
                samples: 3,
                work_units: limits.work_units + limits.work_units / 2,
                scalar_transforms: limits.scalar_transforms + limits.scalar_transforms / 2,
            },
        ),
        Err(SolverError::InvalidPayload)
    ));
    let mut exhausted = BalanceObserver::restore(run.state.plan(), 2, restored.consumption())
        .expect("spent cap restores");
    assert_eq!(
        exhausted.sample(&run.state),
        Err(SolverError::ProviderBudgetExceeded)
    );
}
