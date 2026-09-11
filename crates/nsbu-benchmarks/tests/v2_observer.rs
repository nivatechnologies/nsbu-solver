//! Independent exact-v2 observer resource, sampling and restored-ledger checks.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    smooth_observer::{v2::V2Observer, BalanceObserverWork},
};
use nsbu_solver::{
    domain::{Domain, Epoch, Layout, SpectralState, TickClock},
    SolverError,
};

#[path = "smooth_support/mod.rs"]
mod smooth_support;

fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}
fn settings() -> ForceSettings {
    ForceSettings {
        samples: Layout::new([4; 3]).unwrap(),
        workers: 0,
    }
}

fn settings_m8() -> ForceSettings {
    ForceSettings {
        samples: Layout::new([8; 3]).unwrap(),
        workers: 0,
    }
}

#[test]
fn v2_samples_actual_state_and_doubles_provider_grid() {
    let d = domain();
    let limits = V2Observer::limits(d, settings(), 2).unwrap();
    let run = smooth_support::SmoothRun::new(
        d,
        nsbu_solver::integrators::method::Method::CoxMatthews,
        limits.storage_bytes,
    );
    let state = SpectralState::from_rest(
        run.state.plan(),
        TickClock::from_rest(-10, 8).unwrap(),
        Epoch(0),
    )
    .unwrap();
    let mut observer = V2Observer::new(d, settings(), 2, usize::MAX).unwrap();
    let rest = observer.sample(&state).unwrap();
    assert_eq!(
        rest,
        nsbu_solver::diagnostics::balances::BalanceSample::REST
    );
    observer.sample(&state).unwrap();
    assert_eq!(observer.consumption().samples, 2);
}

#[test]
fn v2_restore_preserves_work_and_rejects_bad_budget() {
    let d = domain();
    let mut observer = V2Observer::new(d, settings(), 2, usize::MAX).unwrap();
    let run = smooth_support::SmoothRun::new(
        d,
        nsbu_solver::integrators::method::Method::CoxMatthews,
        1 << 20,
    );
    let state = SpectralState::from_rest(
        run.state.plan(),
        TickClock::from_rest(-10, 8).unwrap(),
        Epoch(0),
    )
    .unwrap();
    observer.sample(&state).unwrap();
    let work = observer.consumption();
    let mut restored = V2Observer::restore(d, settings(), 2, work, usize::MAX).unwrap();
    assert_eq!(
        restored.sample(&state).unwrap(),
        observer.sample(&state).unwrap()
    );
    assert_eq!(restored.consumption(), observer.consumption());
    assert!(matches!(
        V2Observer::restore(
            d,
            settings(),
            2,
            BalanceObserverWork { samples: 3, ..work },
            usize::MAX
        ),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn v2_mismatched_force_grid_has_explicit_work_bounds() {
    let d = domain();
    let settings = settings_m8();
    let limits = V2Observer::limits(d, settings, 2).unwrap();
    assert!(V2Observer::validate_restored(d, settings, 2, BalanceObserverWork::default()).is_ok());
    assert!(V2Observer::validate_restored(
        d,
        settings,
        2,
        BalanceObserverWork {
            samples: 0,
            work_units: 1,
            scalar_transforms: 0
        },
    )
    .is_err());
    let per_sample_max = limits.work_units / 2;
    let per_sample_transforms = limits.scalar_transforms / 2;
    let min_work = settings.double_grid().unwrap().samples.real_len();
    assert!(V2Observer::validate_restored(
        d,
        settings,
        2,
        BalanceObserverWork {
            samples: 1,
            work_units: min_work,
            scalar_transforms: per_sample_transforms
        },
    )
    .is_ok());
    assert!(V2Observer::validate_restored(
        d,
        settings,
        2,
        BalanceObserverWork {
            samples: 1,
            work_units: min_work - 1,
            scalar_transforms: per_sample_transforms
        },
    )
    .is_err());
    assert!(V2Observer::validate_restored(
        d,
        settings,
        2,
        BalanceObserverWork {
            samples: 1,
            work_units: per_sample_max + 1,
            scalar_transforms: per_sample_transforms
        },
    )
    .is_err());
    assert!(V2Observer::validate_restored(
        d,
        settings,
        2,
        BalanceObserverWork {
            samples: 1,
            work_units: min_work,
            scalar_transforms: per_sample_transforms - 1
        },
    )
    .is_err());
}

#[test]
fn v2_refuses_cap_minus_one_before_construction_and_zero_samples() {
    let d = domain();
    let settings = settings_m8();
    let limits = V2Observer::limits(d, settings, 1).unwrap();
    assert!(matches!(
        V2Observer::new(d, settings, 1, limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    assert!(matches!(
        V2Observer::restore(
            d,
            settings,
            1,
            BalanceObserverWork::default(),
            limits.storage_bytes - 1
        ),
        Err(SolverError::ResourceLimit)
    ));
    assert_eq!(
        V2Observer::limits(d, settings, 0),
        Err(SolverError::ResourceLimit)
    );
}
