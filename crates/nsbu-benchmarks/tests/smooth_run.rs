//! Owned smooth-run snapshots reproduce the next bounded attempt with fresh private scratch.
use nsbu_benchmarks::smooth_run::SmoothRun;
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    SolverError,
};

fn configuration(method: Method, tolerance: f64) -> Configuration {
    Configuration {
        method,
        limits: RunLimits {
            endpoint: 8,
            step_ticks: 4,
            maximum_attempts: 2,
        },
        tolerances: Tolerances {
            absolute: [tolerance; 2],
            relative: [0.0; 2],
        },
    }
}

fn run(method: Method, tolerance: f64) -> SmoothRun {
    SmoothRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-12, 100).unwrap(),
        configuration(method, tolerance),
        2,
        0.3,
        8 * 1024 * 1024,
    )
    .unwrap()
}

fn same_run(left: &SmoothRun, right: &SmoothRun) {
    assert_eq!(left.state().clock(), right.state().clock());
    assert_eq!(
        left.state().accepted_steps(),
        right.state().accepted_steps()
    );
    assert_eq!(left.state().epoch(), right.state().epoch());
    assert_eq!(
        left.history().controller().attempted(),
        right.history().controller().attempted()
    );
    assert_eq!(
        left.history().controller().committed(),
        right.history().controller().committed()
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
        assert_eq!(a.start, b.start);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.sample, b.sample);
    }
    assert_eq!(left.work(), right.work());
    assert_eq!(left.observer_work(), right.observer_work());
    for axis in 0..3 {
        for (a, b) in left
            .state()
            .component(axis)
            .unwrap()
            .iter()
            .zip(right.state().component(axis).unwrap())
        {
            assert_eq!(
                [a.re.to_bits(), a.im.to_bits()],
                [b.re.to_bits(), b.im.to_bits()]
            );
        }
    }
    let a = left.history().balance().integral().unwrap();
    let b = right.history().balance().integral().unwrap();
    assert_eq!(
        [
            a.energy_rhs,
            a.enstrophy_rhs,
            a.energy_defect,
            a.enstrophy_defect
        ]
        .map(f64::to_bits),
        [
            b.energy_rhs,
            b.enstrophy_rhs,
            b.energy_defect,
            b.enstrophy_defect
        ]
        .map(f64::to_bits),
    );
}

#[test]
fn snapshots_resume_cm_and_ho_with_identical_next_commit_and_ledgers() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method, 1.0);
        assert!(matches!(original.step().unwrap(), Outcome::Committed(_)));
        let snapshot = original.snapshot(usize::MAX).unwrap();
        let mut resumed =
            SmoothRun::restore(snapshot, configuration(method, 1.0), 8 * 1024 * 1024).unwrap();
        let expected = original.step().unwrap();
        let actual = resumed.step().unwrap();
        assert_eq!(actual, expected);
        assert!(matches!(actual, Outcome::Committed(_)));
        same_run(&original, &resumed);
        assert_eq!(original.work().len(), 2);
        assert!(original.work().iter().all(|work| work.calls() > 0));
    }
}

#[test]
fn snapshots_resume_the_same_local_rejection_without_reusing_prior_work() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method, 1e-40);
        let snapshot = original.snapshot(usize::MAX).unwrap();
        let mut resumed =
            SmoothRun::restore(snapshot, configuration(method, 1e-40), 8 * 1024 * 1024).unwrap();
        assert!(matches!(original.step().unwrap(), Outcome::Rejected(_)));
        assert_eq!(resumed.step(), Ok(original.history().records()[0].outcome));
        same_run(&original, &resumed);
        assert!(original.work()[0].calls() > 0);
    }
}

#[test]
fn construction_and_restore_refuse_inadequate_or_mismatched_resources() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let config = configuration(Method::CoxMatthews, 1.0);
    assert!(matches!(
        SmoothRun::from_rest(domain, clock, config, 2, 0.3, 1),
        Err(SolverError::ResourceLimit)
    ));
    let run = SmoothRun::from_rest(domain, clock, config, 2, 0.3, 8 * 1024 * 1024).unwrap();
    let snapshot = run.snapshot(usize::MAX).unwrap();
    let mut changed = config;
    changed.limits.maximum_attempts = 1;
    assert!(matches!(
        SmoothRun::restore(snapshot, changed, 8 * 1024 * 1024),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn snapshot_cap_is_preflighted_before_copying_history_or_work() {
    let mut original = run(Method::CoxMatthews, 1.0);
    assert!(matches!(original.step().unwrap(), Outcome::Committed(_)));
    let clock = original.state().clock();
    let attempts = original.history().controller().attempted();
    let work = original.work().to_vec();
    let observer = original.observer_work();
    let required = original.snapshot_reservation().unwrap();
    assert_eq!(
        original.snapshot(required - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    assert_eq!(original.state().clock(), clock);
    assert_eq!(original.history().controller().attempted(), attempts);
    assert_eq!(original.work(), work);
    assert_eq!(original.observer_work(), observer);
    assert!(original.snapshot(required).is_ok());
}

#[test]
fn immutable_profile_admission_and_restore_check_each_fixed_setting() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let config = configuration(Method::CoxMatthews, 1.0);
    assert!(matches!(
        SmoothRun::from_rest(
            domain,
            TickClock::from_rest(-12, 100).unwrap().stages(4).unwrap()[1],
            config,
            2,
            0.3,
            8 * 1024 * 1024,
        ),
        Err(SolverError::InvalidClock)
    ));
    assert!(matches!(
        SmoothRun::from_rest(
            domain,
            TickClock::from_rest(-12, 100).unwrap(),
            config,
            2,
            f64::NAN,
            8 * 1024 * 1024,
        ),
        Err(SolverError::InvalidStep)
    ));
    for changed in [
        Configuration {
            method: Method::HochbruckOstermann,
            ..config
        },
        Configuration {
            limits: RunLimits {
                endpoint: 12,
                ..config.limits
            },
            ..config
        },
        Configuration {
            limits: RunLimits {
                step_ticks: 8,
                ..config.limits
            },
            ..config
        },
        Configuration {
            tolerances: Tolerances {
                absolute: [0.5; 2],
                ..config.tolerances
            },
            ..config
        },
        Configuration {
            tolerances: Tolerances {
                relative: [0.5; 2],
                ..config.tolerances
            },
            ..config
        },
    ] {
        let snapshot = run(Method::CoxMatthews, 1.0).snapshot(usize::MAX).unwrap();
        assert!(matches!(
            SmoothRun::restore(snapshot, changed, 8 * 1024 * 1024),
            Err(SolverError::InvalidPayload)
        ));
    }
    let snapshot = run(Method::CoxMatthews, 1.0).snapshot(usize::MAX).unwrap();
    assert!(matches!(
        SmoothRun::restore(snapshot, config, 1),
        Err(SolverError::ResourceLimit)
    ));
}

#[test]
fn arithmetic_refusal_before_rhs_setup_records_zero_attempt_work() {
    let mut run = SmoothRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-2000, 100).unwrap(),
        configuration(Method::CoxMatthews, 1.0),
        2,
        0.3,
        8 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(
        run.step(),
        Ok(Outcome::Refused {
            cause: SolverError::ArithmeticResolutionLimited,
            indicators: None,
        })
    );
    assert_eq!(run.work()[0].calls(), 0);
    assert_eq!(run.work()[0].work_units(), 0);
    assert_eq!(run.work()[0].scalar_transforms(), 0);
}
