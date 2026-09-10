//! Frobenius tensor normalization, mixed multiplicity and bounded atomic failures.
use nsbu_solver::diagnostics::local::{SampledError, TensorErrors};

#[test]
fn complete_magnitudes_have_the_same_normalization_and_atomic_failure_contract() {
    let mut errors = TensorErrors::<9>::new(2, 0.5).unwrap();
    errors.push_magnitudes(3.0, 4.0).unwrap();
    let before = errors.finish().unwrap();
    for (error, reference) in [
        (-1.0, 0.0),
        (1.0, -1.0),
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
    ] {
        assert!(errors.push_magnitudes(error, reference).is_err());
        assert_eq!(errors.finish().unwrap(), before);
    }
    errors.push_magnitudes(4.0, 0.0).unwrap();
    let SampledError::Measured(report) = errors.finish().unwrap() else {
        panic!("missing data")
    };
    assert_eq!(report.components, 9);
    assert_eq!(report.rms_error, 12.5_f64.sqrt());
    assert_eq!(report.peak_relative_error, 8.0);
    assert!(errors.push_magnitudes(0.0, 0.0).is_err());
    let mut tiny = TensorErrors::<1>::new(1, f64::from_bits(1)).unwrap();
    assert!(tiny.push_magnitudes(1.0, 0.0).is_err());
    assert_eq!(tiny.finish().unwrap(), SampledError::NoSamples);
}

#[test]
fn tensor_magnitude_counts_every_ordered_entry_and_normalizes_only_samples() {
    let mut errors = TensorErrors::<27>::new(2, 0.5).unwrap();
    let mut first = [0.0; 27];
    // d_xy u_x and d_yx u_x are distinct ordered Hessian entries.
    first[1] = 3.0;
    first[3] = 3.0;
    errors.push(first, [0.0; 27]).unwrap();
    errors.push([1.0; 27], [0.0; 27]).unwrap();
    let SampledError::Measured(report) = errors.finish().unwrap() else {
        panic!("missing tensor")
    };
    assert_eq!(report.components, 27);
    assert_eq!(report.samples, 2);
    assert!((report.rms_error - 22.5_f64.sqrt()).abs() < 2e-15);
    assert!((report.peak_error - 27.0_f64.sqrt()).abs() < 2e-15);
    assert_eq!(report.peak_relative_error, 2.0 * report.peak_error);
}

#[test]
fn scalar_pressure_and_gradient_shapes_preserve_floors_and_transactional_refusal() {
    let mut pressure = TensorErrors::<1>::new(1, 0.25).unwrap();
    pressure.push([2.0], [1.5]).unwrap();
    let SampledError::Measured(report) = pressure.finish().unwrap() else {
        panic!("missing pressure")
    };
    assert_eq!(report.components, 1);
    assert_eq!(report.rms_error, 0.5);
    assert_eq!(report.peak_relative_error, 1.0 / 3.0);
    let mut gradient = TensorErrors::<9>::new(2, 1.0).unwrap();
    gradient.push([1.0; 9], [0.0; 9]).unwrap();
    let before = gradient.finish().unwrap();
    let mut bad = [0.0; 9];
    bad[8] = f64::NAN;
    assert!(gradient.push(bad, [0.0; 9]).is_err());
    assert_eq!(gradient.finish().unwrap(), before);
    gradient.push([2.0; 9], [1.0; 9]).unwrap();
    let SampledError::Measured(report) = gradient.finish().unwrap() else {
        panic!("missing gradient")
    };
    assert_eq!(report.rms_error, 3.0);
    assert_eq!(report.reference_peak, 3.0);
    assert!(TensorErrors::<0>::new(1, 1.0).is_err());
    assert!(TensorErrors::<28>::new(1, 1.0).is_err());
}
