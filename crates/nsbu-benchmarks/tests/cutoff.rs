//! Flat regions, periodicity, startup and logistic-tail arithmetic.
use nsbu_benchmarks::{fields, jet::Jet, scalar, time::BenchmarkTime, BenchmarkError};
use nsbu_solver::domain::TickClock;

#[test]
fn logistic_matches_scalar_derivative_and_symmetry() {
    for s in [-1.0, 0.0, 0.001, 0.1, 0.5, 0.9, 0.999, 1.0, 2.0] {
        let (value, derivative) = scalar::smooth_step(s).unwrap();
        let jet = fields::smooth_step(Jet::variable(s, 0).unwrap()).unwrap();
        assert!((jet.value() - value).abs() < 1e-14);
        assert!((jet.derivative(0).unwrap().value() - derivative).abs() < 1e-12);
        assert!((value + scalar::smooth_step(1.0 - s).unwrap().0 - 1.0).abs() < 1e-14);
    }
    assert_eq!(
        scalar::smooth_step(f64::NAN).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(scalar::smooth_step(f64::MIN_POSITIVE).unwrap(), (0.0, 0.0));
}

#[test]
fn periodic_reference_and_force_and_exact_rest() {
    let time = BenchmarkTime::new(TickClock::restore(-10, 8, 4, 4).unwrap()).unwrap();
    let point = [0.125, -0.125, 0.125];
    let shifted = [1.125, 1.875, -2.875];
    let a = fields::evaluate(point, time).unwrap();
    let b = fields::evaluate(shifted, time).unwrap();
    assert_eq!(a.force, b.force);
    assert_eq!(a.velocity, b.velocity);
    assert_eq!(
        scalar::evaluate(point, time).unwrap().velocity,
        scalar::evaluate(shifted, time).unwrap().velocity
    );
    assert_eq!(
        fields::evaluate([f64::NAN, 0.0, 0.0], time).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(
        scalar::evaluate([0.0, f64::INFINITY, 0.0], time).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    let rest = BenchmarkTime::new(TickClock::from_rest(-10, 8).unwrap()).unwrap();
    for (p, t) in [(point, rest), ([0.5, 0.5, 0.5], time)] {
        let zero = fields::evaluate(p, t).unwrap();
        assert_eq!(zero.velocity, [0.0; 3]);
        assert_eq!(zero.force, [0.0; 3]);
        assert_eq!(scalar::evaluate(p, t).unwrap().velocity, [0.0; 3]);
        assert!(zero.root.is_none());
        assert!(scalar::evaluate(p, t).unwrap().root.is_none());
    }
}

#[test]
fn scalar_collar_derivatives_during_startup_match_jets() {
    let time = BenchmarkTime::new(TickClock::restore(-10, 8, 1, 7).unwrap()).unwrap();
    for point in [
        [0.2, 0.2, 0.2],
        [0.1, 0.3, 0.1],
        [0.0, 0.4, 0.0],
        [0.25, -0.25, 0.25],
    ] {
        let scalar = scalar::evaluate(point, time).unwrap();
        let jet = fields::evaluate(point, time).unwrap();
        for (a, b) in scalar.velocity.into_iter().zip(jet.velocity) {
            assert!((a - b).abs() < 1e-12);
        }
        assert_eq!(scalar.root.is_some(), jet.root.is_some());
    }
}
