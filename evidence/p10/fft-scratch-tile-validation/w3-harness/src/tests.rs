//! Tiny no-large-grid controls for the standalone layout-768 harness.
use super::*;
#[test]
fn strict_hermitian_multidirectional_fixture_has_nonzero_projected_advection() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let catalog = catalog(backend).unwrap();
    let bytes = SpectralRhs::<FixtureForce>::reservation_with_fft_backend(
        domain,
        FixtureForce::LIMITS,
        backend,
    )
    .unwrap();
    let mut rhs =
        SpectralRhs::new_with_catalog(domain, FixtureForce { mean: 0.0 }, 1.0e9, &catalog, bytes)
            .unwrap();
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let state = fixture_state(domain).unwrap();
    let mut output = field(domain);
    evaluate_rhs(&mut rhs, &state, clock, &mut output).unwrap();
    let index = domain.layout().locate([0, 1, 1]).unwrap().0;
    let value = output[0][index];
    let expected = -std::f64::consts::PI / 16.0;
    assert!((value.re - expected).abs() < 2.0e-14, "got {:?}", value);
    assert!(value.re.abs() > 0.1);
    assert!(value.im.abs() < 2.0e-14, "got {:?}", value);
}

#[test]
fn planned_whole_control_peaks_fit_the_declared_external_caps() {
    assert!(force_plan().unwrap().peak() <= FORCE_CAP_BYTES);
    assert!(rhs_plan().unwrap().peak() <= RHS_CAP_BYTES);
}

#[test]
fn coefficient_bit_comparison_rejects_length_mismatch() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let actual = field(domain);
    let mut expected = field(domain);
    expected[0].pop();
    assert!(same_bits(&actual, &expected).is_err());
}

#[test]
fn reviewed_768_admission_matches_formulas_and_1024_remains_closed() {
    assert_eq!(
        TILED_FORWARD_ADDITIONAL - UNTILED_FORWARD_ADDITIONAL,
        196_608
    );
    assert_eq!(
        TILED_BIDIRECTIONAL_ADDITIONAL - UNTILED_BIDIRECTIONAL_ADDITIONAL,
        196_608
    );
    let b = FftBackend::RustFft6_4_1AvxFma;
    if b.ensure_available().is_ok() {
        let admitted = Layout::new([LAYOUT; 3]).unwrap();
        let variant = admission().unwrap();
        assert_eq!(
            W3FftPool::additional_reservation_with_backend(admitted, b, W3FftMode::Forward),
            Ok(variant.forward())
        );
        assert_eq!(
            W3FftPool::additional_reservation_with_backend(admitted, b, W3FftMode::Bidirectional),
            Ok(variant.bidirectional())
        );
        let excluded = Layout::new([1024; 3]).unwrap();
        assert_eq!(
            W3FftPool::additional_reservation_with_backend(excluded, b, W3FftMode::Forward),
            Err(SolverError::InvalidPayload)
        );
        assert_eq!(
            W3FftPool::additional_reservation_with_backend(excluded, b, W3FftMode::Bidirectional),
            Err(SolverError::InvalidPayload)
        );
    }
}
