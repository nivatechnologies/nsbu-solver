//! Independent linear-search derivative oracle and complete monomial controls.
use super::{index::POWERS, Jet};
use crate::BenchmarkError;

fn expected(value: Jet, axis: usize) -> [f64; 70] {
    std::array::from_fn(|out| {
        let mut power = POWERS[out];
        power[axis] += 1;
        // Deliberately retain the original search path; do not use the new derivative table.
        match value.coefficient(power) {
            Ok(coefficient) => f64::from(power[axis]) * coefficient,
            Err(_) => 0.0,
        }
    })
}
fn compare(value: Jet) {
    let original = value.coefficients.map(f64::to_bits);
    for axis in 0..4 {
        let derivative = value.derivative(axis).unwrap();
        assert_eq!(
            derivative.coefficients.map(f64::to_bits),
            expected(value, axis).map(f64::to_bits)
        );
    }
    assert_eq!(value.coefficients.map(f64::to_bits), original);
}
#[test]
fn every_monomial_and_mixed_coefficient_word_keeps_the_original_derivative() {
    for i in 0..70 {
        let mut coefficients = [0.0; 70];
        coefficients[i] = -1.25;
        compare(Jet::checked(coefficients).unwrap());
    }
    compare(
        Jet::checked(std::array::from_fn(|i| {
            [0.0, -0.0, 1e-300, -7e-200, 3e200, -0.125, 1.0][i % 7]
        }))
        .unwrap(),
    );
}
#[test]
fn invalid_axis_and_actual_derivative_overflow_remain_structured_refusals() {
    let mut coefficients = [0.0; 70];
    coefficients[Jet::position([4, 0, 0, 0]).unwrap()] = f64::MAX;
    let value = Jet::checked(coefficients).unwrap();
    assert_eq!(
        value.derivative(0),
        Err(BenchmarkError::ArithmeticResolution)
    );
    assert_eq!(value.derivative(4), Err(BenchmarkError::InvalidInput));
    assert_eq!(
        value.derivative(usize::MAX),
        Err(BenchmarkError::InvalidInput)
    );
    assert_eq!(value.derivative(1).unwrap(), Jet::constant(0.0).unwrap());
    assert_eq!(value.coefficient([4, 0, 0, 0]).unwrap(), f64::MAX);
}
