//! Measured integrity failures remain visible; inspecting a field never repairs it.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::integrity::{inspect, SpectrumIntegrity},
    domain::Layout,
    Complex64, SolverError,
};

#[test]
fn valid_fields_and_distinct_mean_and_plane_defects_are_reported() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut field = zero(layout);
    mode(layout, &mut field, [1, 0, 0], [Complex64::new(1.0, 2.0); 3]);
    assert_eq!(
        inspect(layout, slices(&field)).unwrap(),
        SpectrumIntegrity::default()
    );
    let negative = layout.index([3, 0, 0]).unwrap();
    field[1][negative] = Complex64::new(0.5, 3.0);
    let before = field.clone();
    let report = inspect(layout, slices(&field)).unwrap();
    close(report.hermitian_max, 0.5_f64.hypot(5.0));
    assert_eq!(report.nyquist_max, 0.0);
    assert_eq!(report.mean_imaginary_max, 0.0);
    assert_eq!(field, before);
    field[2][0] = Complex64::new(0.0, 3.0);
    let report = inspect(layout, slices(&field)).unwrap();
    assert_eq!(report.hermitian_max, 6.0);
    assert_eq!(report.mean_imaginary_max, 3.0);
}

#[test]
fn the_second_self_conjugate_plane_is_checked_even_though_strict_states_exclude_it() {
    let layout = Layout::new([4, 6, 8]).unwrap();
    let mut field = zero(layout);
    let index = layout.index([1, 2, 4]).unwrap();
    let partner = layout.index([3, 4, 4]).unwrap();
    field[0][index] = Complex64::new(1.0, 2.0);
    let report = inspect(layout, slices(&field)).unwrap();
    close(report.nyquist_max, 5.0_f64.sqrt());
    close(report.hermitian_max, 5.0_f64.sqrt());
    field[0][partner] = field[0][index].conj();
    let report = inspect(layout, slices(&field)).unwrap();
    close(report.nyquist_max, 5.0_f64.sqrt());
    assert_eq!(report.hermitian_max, 0.0);
    field[2][layout.index([2, 0, 1]).unwrap()] = Complex64::new(4.0, 0.0);
    assert_eq!(inspect(layout, slices(&field)).unwrap().nyquist_max, 4.0);
}

#[test]
fn incomplete_nonfinite_and_unrepresentable_defects_are_refused() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut field = zero(layout);
    field[2].pop();
    assert_eq!(
        inspect(layout, slices(&field)).unwrap_err(),
        SolverError::InvalidPayload
    );
    for invalid in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        let mut field = zero(layout);
        field[1][0] = invalid;
        assert_eq!(
            inspect(layout, slices(&field)).unwrap_err(),
            SolverError::InvalidSpectrum
        );
    }
    let mut field = zero(layout);
    field[0][layout.index([3, 0, 0]).unwrap()] = Complex64::new(f64::NAN, 0.0);
    assert_eq!(
        inspect(layout, slices(&field)).unwrap_err(),
        SolverError::InvalidSpectrum
    );
    let mut field = zero(layout);
    field[0][0] = Complex64::new(0.0, f64::MAX);
    assert_eq!(
        inspect(layout, slices(&field)).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}
