//! Algebra identities, mixed derivatives, and explicit arithmetic refusals.
use nsbu_benchmarks::{jet::Jet, BenchmarkError};

#[test]
fn mixed_polynomial_and_transcendental_derivatives() {
    let x = Jet::variable(2.0, 0).unwrap();
    let y = Jet::variable(3.0, 1).unwrap();
    let product = x.times(x).unwrap().times(y).unwrap().times(y).unwrap();
    assert_eq!(product.value(), 36.0);
    assert_eq!(product.coefficient([2, 2, 0, 0]).unwrap(), 1.0);
    assert_eq!(
        product
            .derivative(0)
            .unwrap()
            .derivative(1)
            .unwrap()
            .value(),
        24.0
    );
    let quotient = product.quotient(y.powf(2.0).unwrap()).unwrap();
    for (actual, expected) in quotient
        .coefficients()
        .iter()
        .zip(x.times(x).unwrap().coefficients())
    {
        assert!((actual - expected).abs() < 1e-13);
    }
    let power = x.powf(0.25).unwrap();
    let fourth = power.times(power).unwrap().powf(2.0).unwrap();
    for (actual, expected) in fourth.coefficients().iter().zip(x.coefficients()) {
        assert!((actual - expected).abs() < 2e-14);
    }
    let exponential = x.plus(y).unwrap().exp().unwrap();
    assert!((exponential.coefficient([2, 2, 0, 0]).unwrap() - 5.0_f64.exp() / 4.0).abs() < 1e-13);
    assert_eq!(x.powf(0.0).unwrap(), Jet::constant(1.0).unwrap());
    assert_eq!(product.minus(product).unwrap(), Jet::constant(0.0).unwrap());
}

#[test]
fn tail_recovery_and_total_underflow() {
    let tail = Jet::variable(-750.0, 0)
        .unwrap()
        .plus(Jet::variable(0.0, 1).unwrap().scale(1e10).unwrap())
        .unwrap()
        .exp()
        .unwrap();
    assert_eq!(tail.value(), 0.0);
    assert!(tail.coefficient([0, 1, 0, 0]).unwrap() > 0.0);
    assert_eq!(
        Jet::variable(-1e6, 0).unwrap().exp().unwrap(),
        Jet::constant(0.0).unwrap()
    );
    assert_eq!(
        Jet::constant(-750.0).unwrap().exp().unwrap(),
        Jet::constant(0.0).unwrap()
    );
}

#[test]
fn invalid_indices_domains_and_overflow_are_refused() {
    let one = Jet::constant(1.0).unwrap();
    assert_eq!(
        Jet::constant(f64::NAN).unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
    assert_eq!(
        Jet::variable(0.0, 4).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(one.derivative(4).unwrap_err(), BenchmarkError::InvalidInput);
    assert_eq!(
        one.coefficient([255, 0, 0, 0]).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(
        one.powf(f64::INFINITY).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(
        Jet::constant(0.0).unwrap().powf(2.0).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    assert_eq!(
        one.scale(f64::INFINITY).unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
    assert_eq!(
        Jet::constant(1000.0).unwrap().exp().unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
    let large = Jet::constant(f64::MAX).unwrap();
    assert_eq!(
        large.plus(large).unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
    assert_eq!(
        large.times(large).unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
}

#[test]
fn coefficient_view_is_nonzero_and_preserves_degree_order() {
    let x = Jet::variable(2.0, 0).unwrap();
    assert_eq!(x.coefficients()[0], 2.0);
    assert_eq!(x.coefficients().iter().sum::<f64>(), 3.0);
    let power = Jet::variable(1e100, 0).unwrap().powf(-4.0).unwrap();
    assert_eq!(power.value(), 0.0);
}
