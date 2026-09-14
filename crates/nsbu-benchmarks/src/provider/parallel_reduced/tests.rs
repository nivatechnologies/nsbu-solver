//! Private force-provider failure and compatibility controls.
use super::*;

fn clock(tick: u128) -> TickClock {
    TickClock::restore(-10, 8, tick, 8 - tick).unwrap()
}

fn provider() -> (ParallelReducedV2Force, ForceLimits, [Vec<Complex64>; 3]) {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6, 8, 12]).unwrap();
    let limits = ParallelReducedV2Force::preflight(domain, samples, 3).unwrap();
    (
        ParallelReducedV2Force::new(domain, samples, 3, limits.storage_bytes).unwrap(),
        limits,
        std::array::from_fn(|_| vec![Complex64::new(13.0, -9.0); domain.layout().half_len()]),
    )
}

#[test]
fn worker_error_drains_the_attempt_and_preserves_the_previous_output() {
    for failed in 0..3 {
        let (mut provider, limits, mut output) = provider();
        provider
            .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        let prior = output.clone();
        let roots = provider.last_root_iterations();
        provider.pool.fail_next(failed);
        assert_eq!(
            provider
                .evaluate(clock(4), limits, output.each_mut().map(Vec::as_mut_slice))
                .unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
        assert_eq!(output, prior);
        assert_eq!(provider.last_root_iterations(), roots);
        assert!(provider.is_terminated());
        assert!(provider.pool.all_collected());
    }
}

#[test]
fn worker_panic_drains_the_attempt_and_preserves_unpublished_output() {
    let (mut provider, limits, mut output) = provider();
    let prior = output.clone();
    provider.pool.panic_next(1);
    assert_eq!(
        provider
            .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    assert_eq!(output, prior);
    assert!(provider.is_terminated());
    assert!(provider.pool.all_collected());
}

#[test]
fn opt_in_w3_force_charges_exact_experiment_addition() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let domain = Domain::new([256; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([384; 3]).unwrap();
    let workers = 12;
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, workers, backend)
            .unwrap();
    let w3_limits =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, workers, backend)
            .unwrap();
    let addition = nsbu_solver::spectral::W3FftPool::additional_reservation_with_backend(
        samples,
        backend,
        nsbu_solver::spectral::W3FftMode::Forward,
    )
    .unwrap();
    assert_eq!(
        w3_limits.storage_bytes - serial_limits.storage_bytes,
        addition
    );
}

#[test]
fn fixture_w3_force_matches_serial_and_refuses_after_fft_failure() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, 3, backend)
            .unwrap();
    let w3_limits =
        ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, 3, backend)
            .unwrap();
    let mut serial = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        3,
        &catalog,
        serial_limits.storage_bytes,
    )
    .unwrap();
    let mut w3 = ParallelReducedV2ForceW3::new_with_catalog(
        domain,
        samples,
        3,
        &catalog,
        w3_limits.storage_bytes,
    )
    .unwrap();
    let mut serial_output =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut w3_output = serial_output.clone();
    let serial_work = serial
        .evaluate(
            clock(1),
            serial_limits,
            serial_output.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let w3_work = w3
        .evaluate(
            clock(1),
            w3_limits,
            w3_output.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    assert_eq!(w3_output, serial_output);
    assert_eq!(w3_work.work_units, serial_work.work_units);
    assert_eq!(w3_work.scalar_transforms, serial_work.scalar_transforms);

    let prior_iterations = w3.inner.last_root_iterations;
    w3.inner.physical[0][0] = f64::NAN;
    let sentinel = Complex64::new(17.0, -19.0);
    let mut unpublished = std::array::from_fn(|_| vec![sentinel; domain.layout().half_len()]);
    assert_eq!(
        w3.inner
            .transform(unpublished.each_mut().map(Vec::as_mut_slice)),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    assert!(unpublished.iter().flatten().all(|value| *value == sentinel));
    assert!(w3.is_terminated());
    assert_eq!(
        w3.evaluate(
            clock(4),
            w3_limits,
            unpublished.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap_err(),
        SolverError::ProviderBudgetExceeded
    );
    assert_eq!(w3.inner.last_root_iterations, prior_iterations);
    assert!(unpublished.iter().flatten().all(|value| *value == sentinel));
}
