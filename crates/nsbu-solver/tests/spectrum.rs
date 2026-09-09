//! Payload invariants are checked without projection, clipping or mutation.
use nsbu_solver::domain::{validate_spectrum, Layout};
use nsbu_solver::{Complex64, SolverError};

#[test]
fn arbitrary_positive_slice_and_conjugate_zero_plane_are_valid() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut values = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    values[0] = Complex64::new(3.0, 0.0);
    values[layout.index([1, 1, 0]).unwrap()] = Complex64::new(2.0, 5.0);
    values[layout.index([3, 3, 0]).unwrap()] = Complex64::new(2.0, -5.0);
    values[layout.index([1, 1, 1]).unwrap()] = Complex64::new(-7.0, 9.0);
    let saved = values.clone();
    assert_eq!(validate_spectrum(layout, &values, 0.0), Ok(()));
    assert_eq!(values, saved);
    values[0].im = 0.25;
    assert_eq!(validate_spectrum(layout, &values, 0.5), Ok(()));
    assert_eq!(
        validate_spectrum(layout, &values, 0.49),
        Err(SolverError::InvalidSpectrum)
    );
}

#[test]
fn invalid_payload_tolerance_and_nonfinite_coefficients_are_refused() {
    let layout = Layout::new([4; 3]).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for tolerance in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            validate_spectrum(layout, &zero, tolerance),
            Err(SolverError::InvalidPayload)
        );
    }
    assert_eq!(
        validate_spectrum(layout, &zero[..47], 0.0),
        Err(SolverError::InvalidPayload)
    );
    let oversized = vec![Complex64::new(0.0, 0.0); 49];
    assert_eq!(
        validate_spectrum(layout, &oversized, 0.0),
        Err(SolverError::InvalidPayload)
    );
    for value in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        let mut values = zero.clone();
        values[1] = value;
        assert_eq!(
            validate_spectrum(layout, &values, 0.0),
            Err(SolverError::InvalidSpectrum)
        );
    }
}

#[test]
fn nyquist_and_nonhermitian_inputs_are_refused_even_with_large_tolerance() {
    let layout = Layout::new([4; 3]).unwrap();
    for position in [[2, 0, 0], [0, 2, 0], [0, 0, 2]] {
        let mut values = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        values[layout.index(position).unwrap()] = Complex64::new(f64::MIN_POSITIVE, 0.0);
        assert_eq!(
            validate_spectrum(layout, &values, 1.0),
            Err(SolverError::InvalidSpectrum)
        );
    }
    let mut values = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    values[layout.index([1, 3, 0]).unwrap()] = Complex64::new(0.5, 0.0);
    assert_eq!(validate_spectrum(layout, &values, 0.5), Ok(()));
    assert_eq!(
        validate_spectrum(layout, &values, 0.49),
        Err(SolverError::InvalidSpectrum)
    );
    values[layout.index([3, 1, 0]).unwrap()] = Complex64::new(0.5, 0.0);
    assert_eq!(validate_spectrum(layout, &values, 0.0), Ok(()));
}
