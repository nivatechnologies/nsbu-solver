//! Bounded local errors, explicit absence, relative floors and transactional rejection.
use nsbu_solver::{
    diagnostics::local::{ErrorAccumulator, LocalError, SampledError},
    SolverError,
};

fn measurement(sample: SampledError) -> Option<LocalError> {
    match sample {
        SampledError::NoSamples => None,
        SampledError::Measured(value) => Some(value),
    }
}

#[test]
fn sampled_vector_errors_retain_absolute_and_floored_relative_information() {
    let mut accumulator = ErrorAccumulator::new(3, 0.5).unwrap();
    assert_eq!(accumulator.finish().unwrap(), SampledError::NoSamples);
    assert_eq!(measurement(accumulator.finish().unwrap()), None);
    accumulator.push([3.0, 4.0, 0.0], [0.0; 3]).unwrap();
    accumulator.push([6.0, 8.0, 0.0], [3.0, 4.0, 0.0]).unwrap();
    accumulator
        .push([3.0, 4.0, 12.0], [3.0, 4.0, 10.0])
        .unwrap();
    let before = accumulator.finish().unwrap();
    let report = measurement(before).unwrap();
    assert_eq!(report.samples, 3);
    assert!((report.rms_error - 18.0_f64.sqrt()).abs() < 2e-15);
    assert_eq!(report.peak_error, 5.0);
    assert_eq!(report.peak_relative_error, 10.0);
    assert_eq!(report.reference_peak, 125.0_f64.sqrt());
    assert_eq!(report.relative_floor, 0.5);
    assert_eq!(
        accumulator.push([0.0; 3], [0.0; 3]).unwrap_err(),
        SolverError::ResourceLimit
    );
    assert_eq!(accumulator.finish().unwrap(), before);
}

#[test]
fn failed_arithmetic_preserves_every_prior_measurement() {
    let mut accumulator = ErrorAccumulator::new(3, 1.0).unwrap();
    accumulator.push([1.0, 0.0, 0.0], [0.0; 3]).unwrap();
    let before = accumulator.finish().unwrap();
    for (actual, reference, error) in [
        ([f64::NAN, 0.0, 0.0], [0.0; 3], SolverError::InvalidSpectrum),
        (
            [0.0; 3],
            [0.0, f64::INFINITY, 0.0],
            SolverError::InvalidSpectrum,
        ),
        (
            [f64::MAX, 0.0, 0.0],
            [-f64::MAX, 0.0, 0.0],
            SolverError::ArithmeticResolutionLimited,
        ),
        (
            [f64::MAX, f64::MAX, 0.0],
            [f64::MAX, f64::MAX, 0.0],
            SolverError::ArithmeticResolutionLimited,
        ),
    ] {
        assert_eq!(accumulator.push(actual, reference).unwrap_err(), error);
        assert_eq!(accumulator.finish().unwrap(), before);
    }
    let mut tiny = ErrorAccumulator::new(1, f64::from_bits(1)).unwrap();
    assert_eq!(
        tiny.push([1.0; 3], [0.0; 3]).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    assert_eq!(tiny.finish().unwrap(), SampledError::NoSamples);
}

#[test]
fn normalization_precedes_an_overflowing_unnormalized_norm() {
    let mut accumulator = ErrorAccumulator::new(16, 1e308).unwrap();
    for _ in 0..16 {
        accumulator.push([1e308, 0.0, 0.0], [0.0; 3]).unwrap();
    }
    let report = measurement(accumulator.finish().unwrap()).unwrap();
    assert_eq!(report.rms_error, 1e308);
    assert_eq!(report.peak_relative_error, 1.0);
}

#[test]
fn invalid_capacity_and_denominator_floors_are_refused() {
    assert!(matches!(
        ErrorAccumulator::new(0, 1.0),
        Err(SolverError::InvalidPayload)
    ));
    for floor in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(matches!(
            ErrorAccumulator::new(1, floor),
            Err(SolverError::InvalidPayload)
        ));
    }
}
