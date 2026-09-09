//! Exact dyadic-to-binary64 boundaries, including subnormal intervals.
use nsbu_solver::{integrators::time::binary_duration, SolverError};

#[test]
fn exact_intervals_preserve_low_bits_and_refuse_arithmetic_loss() {
    for (ticks, exponent, expected) in [
        (8, -6, 0.125),
        (1, -1074, f64::from_bits(1)),
        (1, -1073, f64::from_bits(2)),
        (1, -1050, f64::from_bits(1 << 24)),
        (3, -1074, f64::from_bits(3)),
        (1, -1022, f64::MIN_POSITIVE),
        (1, 1023, f64::from_bits(2046_u64 << 52)),
        (1_u128 << 127, -127, 1.0),
        ((1_u128 << 53) - 1, -52, 2.0 - f64::EPSILON),
    ] {
        assert_eq!(binary_duration(ticks, exponent), Ok(expected));
    }
    assert_eq!(binary_duration(0, 0), Err(SolverError::InvalidStep));
    for (ticks, exponent) in [
        (1, i32::MIN),
        (1, i32::MAX),
        (1, -1075),
        (3, 1023),
        ((1_u128 << 53) + 1, -53),
        (u128::MAX, 0),
    ] {
        assert_eq!(
            binary_duration(ticks, exponent),
            Err(SolverError::ArithmeticResolutionLimited)
        );
    }
}
