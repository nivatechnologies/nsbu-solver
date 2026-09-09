//! Exact-clock separation and safeguarded implicit-root behavior.
use nsbu_benchmarks::{fields, jet::Jet, root, time::BenchmarkTime, BenchmarkError};
use nsbu_solver::domain::TickClock;

#[test]
fn remaining_survives_rounded_target_elapsed() {
    let target = 1_u128 << 127;
    let time =
        BenchmarkTime::new(TickClock::restore(-134, target, target - 1, 1).unwrap()).unwrap();
    assert_eq!(time.elapsed(), 1.0 / 128.0);
    assert_eq!(time.remaining(), 2.0_f64.powi(-134));
    assert!(time.rounding_estimates()[0] > 0.0);
    assert_eq!(time.rounding_estimates()[1], 0.0);
    let rest = BenchmarkTime::new(TickClock::from_rest(-10, 8).unwrap()).unwrap();
    assert_eq!(rest.elapsed(), 0.0);
    assert_eq!(rest.rounding_estimates(), [0.0, 0.0]);
    assert_eq!(
        BenchmarkTime::new(TickClock::from_rest(-10, 7).unwrap()).unwrap_err(),
        BenchmarkError::ClockIdentity
    );
    assert_eq!(
        BenchmarkTime::new(TickClock::from_rest(-9, 8).unwrap()).unwrap_err(),
        BenchmarkError::ClockIdentity
    );
}

#[test]
fn roots_have_small_residuals_and_bounded_work() {
    for tau in [1.0 / 128.0, 1e-10, 2.0_f64.powi(-134)] {
        for z in [0.0, 1e-20, 0.001, -0.1, 0.5] {
            let report = root::solve(z, tau, 128).unwrap();
            assert!(report.value >= report.bracket[0]);
            assert!(report.value <= report.bracket[1]);
            assert!(report.residual.abs() <= 8.0 * f64::EPSILON * report.value);
            assert!(report.error_estimate >= report.residual.abs());
            assert!(report.error_estimate <= 5e-15 * report.value);
            assert!(report.iterations <= 128);
        }
    }
    assert_eq!(
        root::solve(0.1, 1.0 / 256.0, 1).unwrap_err(),
        BenchmarkError::RootWorkExhausted
    );
    for (z, tau, limit) in [
        (f64::NAN, 0.001, 128),
        (0.6, 0.001, 128),
        (0.0, f64::NAN, 128),
        (0.0, 0.0, 128),
        (0.0, 0.1, 128),
        (0.0, 0.001, 0),
        (0.0, 0.001, 129),
    ] {
        assert_eq!(
            root::solve(z, tau, limit).unwrap_err(),
            BenchmarkError::InvalidInput
        );
    }
}

#[test]
fn implicit_axis_mixed_derivatives_obey_equation() {
    let time = BenchmarkTime::new(TickClock::restore(-10, 8, 4, 4).unwrap()).unwrap();
    let (q, residual, report) =
        fields::implicit_root(Jet::variable(0.0, 2).unwrap(), time).unwrap();
    assert_eq!(report.iterations, 0);
    assert_eq!(q.coefficient([0, 0, 0, 1]).unwrap(), -1.0);
    assert_eq!(
        q.coefficient([0, 0, 2, 0]).unwrap(),
        time.remaining().powf(0.25)
    );
    let mixed = q.coefficient([0, 0, 2, 1]).unwrap();
    assert!((mixed + 0.25 * time.remaining().powf(-0.75)).abs() < 1e-12);
    for value in residual.coefficients() {
        assert!(value.abs() < 1e-10);
    }
}

#[test]
fn conversion_error_is_scaled_to_each_coordinate() {
    let target = 1_u128 << 100;
    let time =
        BenchmarkTime::new(TickClock::restore(-107, target, 1, target - 1).unwrap()).unwrap();
    assert_eq!(time.rounding_estimates(), [0.0, f64::EPSILON / 128.0]);
    let exact =
        BenchmarkTime::new(TickClock::restore(-107, target, target / 2, target / 2).unwrap())
            .unwrap();
    assert_eq!(exact.rounding_estimates(), [0.0, 0.0]);
}

#[test]
fn trailing_zero_removal_detects_exact_large_odd_significands() {
    let elapsed = ((1_u128 << 52) + 1) << 20;
    let target = 1_u128 << 100;
    let time =
        BenchmarkTime::new(TickClock::restore(-107, target, elapsed, target - elapsed).unwrap())
            .unwrap();
    assert_eq!(time.rounding_estimates()[0], 0.0);
}

#[test]
fn safeguarded_newton_meets_small_observed_work_budgets() {
    assert_eq!(root::solve(0.01, 0.001, 4).unwrap().iterations, 4);
    assert_eq!(root::solve(0.1, 0.001, 5).unwrap().iterations, 5);
    assert_eq!(root::solve(0.5, 0.001, 5).unwrap().iterations, 5);
}

#[test]
fn full_evaluator_handles_both_extreme_positive_clock_coordinates() {
    let target = 1_u128 << 127;
    let first =
        BenchmarkTime::new(TickClock::restore(-134, target, 1, target - 1).unwrap()).unwrap();
    let startup = fields::evaluate([0.0; 3], first).unwrap();
    assert_eq!(startup.velocity, [0.0; 3]);
    assert_eq!(startup.force, [0.0; 3]);
    let last =
        BenchmarkTime::new(TickClock::restore(-134, target, target - 1, 1).unwrap()).unwrap();
    let sample = fields::evaluate([0.0; 3], last).unwrap();
    let expected = last.remaining().powf(-5.0 / 8.0) / 32.0;
    assert!((sample.velocity[2] / expected - 1.0).abs() < 1e-14);
    assert!(sample.force.iter().all(|v| v.is_finite()));
    assert!(sample
        .force_gradient
        .iter()
        .flatten()
        .all(|v| v.is_finite()));
}
