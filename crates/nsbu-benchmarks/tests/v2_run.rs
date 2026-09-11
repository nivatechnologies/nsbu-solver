//! Focused exact-v2 owned runtime checks.
mod fixture_support;

use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    SolverError,
};

fn settings(method: Method, workers: usize, tolerance: f64, attempts: usize) -> Settings {
    Settings {
        domain: Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method,
            limits: RunLimits {
                endpoint: 4096,
                step_ticks: 128,
                maximum_attempts: attempts,
            },
            tolerances: Tolerances {
                absolute: if tolerance == 1e-5 {
                    [1e-5, 1e-4]
                } else {
                    [tolerance; 2]
                },
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn run(settings: Settings) -> Run {
    let plan = Plan::from_rest(settings, 1 << 26).unwrap();
    Run::from_rest(plan).unwrap()
}

fn compare_fixture(run: &Run, fixture: &str) {
    let output = std::array::from_fn(|axis| run.state().component(axis).unwrap().to_vec());
    fixture_support::compare(
        run.state().plan().domain().layout(),
        &output,
        fixture,
        5e-13,
    );
}

fn same_records(
    left: &[nsbu_solver::experiment::log::AttemptRecord],
    right: &[nsbu_solver::experiment::log::AttemptRecord],
) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(a.start, b.start);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.sample, b.sample);
    }
}

#[test]
fn exact_v2_cm_and_ho_reach_endpoint_and_match_full_band_fixtures() {
    for (method, fixture) in [
        (
            Method::CoxMatthews,
            include_str!("fixtures/concentrating-n4.tsv"),
        ),
        (
            Method::HochbruckOstermann,
            include_str!("fixtures/concentrating-ho-n4.tsv"),
        ),
    ] {
        let mut run = run(settings(method, 0, 1e-5, 32));
        for step in 0..32 {
            let outcome = run.step().unwrap();
            assert!(
                matches!(outcome, Outcome::Committed(_)),
                "step {step}: {outcome:?}"
            );
        }
        assert_eq!(run.state().clock().remaining(), 4096);
        assert_eq!(run.state().accepted_steps(), 32);
        assert_eq!(run.history().records().len(), 32);
        assert_eq!(run.work().len(), 32);
        let calls = method.rhs_calls();
        assert!(run
            .work()
            .iter()
            .all(|entry| entry.integration()[0] == calls));
        assert!(run
            .work()
            .iter()
            .all(|entry| entry.integration()[2] == 13 * calls));
        assert!(run
            .work()
            .iter()
            .all(|entry| entry.observation().samples == 1));
        assert_eq!(run.observer_work().samples, 32);
        assert_eq!(run.observer_work().scalar_transforms, 32 * 12);
        compare_fixture(&run, fixture);
        println!(
            "exact-v2 method={method:?} steps=32 calls={calls} observer_transforms={}",
            run.observer_work().scalar_transforms
        );
    }
}

#[test]
fn serial_and_parallel_runtime_have_identical_words() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut serial = run(settings(method, 0, 1e-5, 32));
        let mut parallel = run(settings(method, 2, 1e-5, 32));
        for _ in 0..2 {
            assert_eq!(serial.step().unwrap(), parallel.step().unwrap());
        }
        for axis in 0..3 {
            let a = serial.state().component(axis).unwrap();
            let b = parallel.state().component(axis).unwrap();
            assert!(
                a.iter()
                    .zip(b)
                    .all(|(x, y)| x.re.to_bits() == y.re.to_bits()
                        && x.im.to_bits() == y.im.to_bits())
            );
        }
        same_records(serial.history().records(), parallel.history().records());
        assert_eq!(serial.work(), parallel.work());
    }
}

#[test]
fn low_tolerance_rejection_and_terminal_retry_preserve_committed_state_and_work() {
    let mut run = run(settings(Method::CoxMatthews, 0, 1e-40, 32));
    let before: Vec<_> = (0..3)
        .map(|axis| run.state().component(axis).unwrap().to_vec())
        .collect();
    let clock = run.state().clock();
    let outcome = run.step().unwrap();
    assert!(matches!(outcome, Outcome::Rejected(_)));
    assert_eq!(run.state().clock(), clock);
    assert_eq!(run.state().accepted_steps(), 0);
    for (axis, values) in before.iter().enumerate() {
        assert_eq!(run.state().component(axis).unwrap(), values);
    }
    assert_eq!(run.work().len(), 1);
    let work = run.work().to_vec();
    let history = run
        .history()
        .records()
        .iter()
        .map(|record| (record.start, record.outcome, record.sample))
        .collect::<Vec<_>>();
    assert_eq!(run.step().unwrap_err(), SolverError::RetryLimit);
    assert_eq!(run.work(), work);
    assert_eq!(run.history().records().len(), history.len());
    for (record, (start, outcome, sample)) in run.history().records().iter().zip(history) {
        assert_eq!(
            (record.start, record.outcome, record.sample),
            (start, outcome, sample)
        );
    }
    assert_eq!(run.state().clock(), clock);
}

#[test]
fn guard_refusal_preserves_state_and_records_only_the_refused_attempt() {
    let mut config = settings(Method::CoxMatthews, 0, 1e-5, 32);
    config.advective_limit = 1e-30;
    let mut run = run(config);
    let clock = run.state().clock();
    let accepted = run.state().accepted_steps();
    let before: Vec<_> = (0..3)
        .map(|axis| {
            run.state()
                .component(axis)
                .unwrap()
                .iter()
                .map(|value| (value.re.to_bits(), value.im.to_bits()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(matches!(
        run.step().unwrap(),
        Outcome::Refused {
            cause: SolverError::AdvectiveLimit,
            ..
        }
    ));
    assert_eq!(run.state().clock(), clock);
    assert_eq!(run.state().accepted_steps(), accepted);
    for (axis, expected) in before.iter().enumerate() {
        let actual = run
            .state()
            .component(axis)
            .unwrap()
            .iter()
            .map(|value| (value.re.to_bits(), value.im.to_bits()))
            .collect::<Vec<_>>();
        assert_eq!(&actual, expected);
    }
    assert_eq!(run.work().len(), 1);
    assert!(run.work()[0].integration()[0] > 0);
    assert!(run.work()[0].integration()[0] <= Method::CoxMatthews.rhs_calls());
    assert_eq!(run.work()[0].observation().samples, 0);
}

#[test]
fn preflight_rejects_malformed_settings_without_allocating_a_run() {
    let valid = settings(Method::CoxMatthews, 0, 1e-5, 32);
    assert!(matches!(
        Plan::from_rest(
            Settings {
                domain: Domain::new([4; 3], [2.0; 3], 1.0).unwrap(),
                ..valid
            },
            1 << 26
        ),
        Err(SolverError::InvalidDomain)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                advective_limit: 0.0,
                ..valid
            },
            1 << 26
        ),
        Err(SolverError::InvalidStep)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                advective_limit: f64::NAN,
                ..valid
            },
            1 << 26
        ),
        Err(SolverError::InvalidStep)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                initial_clock: TickClock::restore(-20, 8192, 1, 8191).unwrap(),
                ..valid
            },
            1 << 26
        ),
        Err(SolverError::InvalidClock)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                configuration: Configuration {
                    limits: RunLimits {
                        maximum_attempts: 0,
                        ..valid.configuration.limits
                    },
                    ..valid.configuration
                },
                ..valid
            },
            1 << 26
        ),
        Err(SolverError::RetryLimit)
    ));
    assert!(matches!(
        Plan::from_rest(valid, 0),
        Err(SolverError::ResourceLimit)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                configuration: Configuration {
                    limits: RunLimits {
                        endpoint: u128::MAX,
                        ..valid.configuration.limits
                    },
                    ..valid.configuration
                },
                ..valid
            },
            usize::MAX
        ),
        Err(SolverError::InvalidClock)
    ));
    assert!(matches!(
        Plan::from_rest(
            Settings {
                configuration: Configuration {
                    limits: RunLimits {
                        maximum_attempts: usize::MAX,
                        ..valid.configuration.limits
                    },
                    ..valid.configuration
                },
                ..valid
            },
            usize::MAX
        ),
        Err(SolverError::RetryLimit)
    ));
}
