//! Direct polynomial/series checks independent of the compiled product and derivative tables.
use super::{
    index::{DERIVATIVES, POWERS, PRODUCTS},
    Jet,
};
use crate::BenchmarkError;
#[test]
fn complete_monomial_products_and_derivatives_match_direct_multi_indices() {
    let mut products = 0;
    for (a, pa) in POWERS.iter().enumerate() {
        let mut left = Jet::constant(0.0).unwrap();
        left.coefficients[a] = 1.0;
        for (b, pb) in POWERS.iter().enumerate() {
            let mut right = Jet::constant(0.0).unwrap();
            right.coefficients[b] = 1.0;
            let power = std::array::from_fn(|i| pa[i] + pb[i]);
            let mut expected = [0.0; 20];
            if let Some(index) = POWERS.iter().position(|p| *p == power) {
                expected[index] = 1.0;
                products += 1;
            }
            assert_eq!(left.times(right).unwrap().coefficients, expected);
        }
        for axis in 0..3 {
            let mut expected = [0.0; 20];
            if pa[axis] > 0 {
                let mut power = *pa;
                power[axis] -= 1;
                expected[POWERS.iter().position(|p| *p == power).unwrap()] = f64::from(pa[axis]);
            }
            assert_eq!(left.derivative(axis).unwrap().coefficients, expected);
        }
    }
    assert_eq!(products, 84);
    assert_eq!(PRODUCTS.len(), 84);
    assert_eq!(DERIVATIVES.len(), 3);
}
#[test]
fn mixed_compositions_match_a_separate_degree_four_cartesian_algebra() {
    let reduced = Jet::variable(1.5, 0)
        .unwrap()
        .plus(Jet::variable(0.0, 1).unwrap().scale(0.25).unwrap())
        .unwrap()
        .plus(Jet::variable(0.0, 2).unwrap().scale(-0.125).unwrap())
        .unwrap();
    let full = crate::jet::Jet::variable(1.5, 0)
        .unwrap()
        .plus(
            crate::jet::Jet::variable(0.0, 2)
                .unwrap()
                .scale(0.25)
                .unwrap(),
        )
        .unwrap()
        .plus(
            crate::jet::Jet::variable(0.0, 3)
                .unwrap()
                .scale(-0.125)
                .unwrap(),
        )
        .unwrap();
    for (a, b) in [
        (reduced.exp().unwrap(), full.exp().unwrap()),
        (reduced.powf(-0.625).unwrap(), full.powf(-0.625).unwrap()),
        (
            reduced.quotient(reduced).unwrap(),
            full.quotient(full).unwrap(),
        ),
        (reduced.powf(2.0).unwrap(), full.powf(2.0).unwrap()),
    ] {
        for power in POWERS {
            let expected = b.coefficient([power[0], 0, power[1], power[2]]).unwrap();
            let actual = a.coefficient(power).unwrap();
            assert!((actual - expected).abs() < 1e-14 * (1.0 + expected.abs()));
        }
    }
}
#[test]
fn logarithmic_underflow_recovery_and_arithmetic_refusals_are_explicit() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Jet::constant(value).is_err());
    }
    let one = Jet::constant(1.0).unwrap();
    assert!(Jet::variable(0.0, 3).is_err());
    assert!(one.derivative(3).is_err());
    assert!(one.coefficient([4, 0, 0]).is_err());
    assert!(one.powf(f64::NAN).is_err());
    assert!(Jet::constant(0.0).unwrap().powf(-1.0).is_err());
    let huge = Jet::constant(f64::MAX).unwrap();
    assert!(huge.plus(huge).is_err());
    assert!(huge.minus(huge.scale(-1.0).unwrap()).is_err());
    assert!(huge.scale(2.0).is_err());
    assert!(huge.times(huge).is_err());
    let mut extreme = one;
    extreme.coefficients[19] = f64::MAX;
    assert!(extreme.derivative(0).is_err());
    let suppressed = Jet::constant(-1000.0).unwrap().exp().unwrap();
    assert_eq!(suppressed.value(), 0.0);
    let recover = Jet::variable(0.0, 0)
        .unwrap()
        .scale(1e100)
        .unwrap()
        .plus(Jet::constant(-750.0).unwrap())
        .unwrap()
        .exp()
        .unwrap();
    assert_eq!(recover.value(), 0.0);
    assert!(recover.coefficient([3, 0, 0]).unwrap() > 0.0);
    let large = Jet::variable(0.0, 0).unwrap().scale(1e110).unwrap();
    let bound = large.exponential_log_bound();
    // Form neighboring representable inputs around the computed logarithmic threshold.
    // Subtracting the majorant need not land exactly on -750 in binary64.
    let constant = (-750.0 - bound).next_up();
    let boundary = large.plus(Jet::constant(constant).unwrap()).unwrap();
    assert!(boundary.exponential_log_bound() >= -750.0);
    assert_eq!(
        boundary.exp().unwrap_err(),
        BenchmarkError::ArithmeticResolution
    );
    let below = large
        .plus(Jet::constant(constant.next_down()).unwrap())
        .unwrap();
    assert!(below.exponential_log_bound() < -750.0);
    assert_eq!(below.exp().unwrap(), Jet::constant(0.0).unwrap());
}
