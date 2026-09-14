//! Parallel reduced sampling preserves the complete serial reduced spectrum and request contract.
use nsbu_benchmarks::provider::{
    parallel_reduced::ParallelReducedV2Force, reduced::ReducedV2Force,
};
use nsbu_solver::{
    domain::{validate_spectrum, Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};

fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}

fn output(domain: Domain) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()])
}

#[test]
fn every_parallel_reduced_coefficient_matches_the_serial_reduced_order() {
    let domain = domain();
    let samples = Layout::new([6, 8, 12]).unwrap();
    let serial_limits = ReducedV2Force::preflight(domain, samples).unwrap();
    let mut serial = ReducedV2Force::new(domain, samples, serial_limits.storage_bytes).unwrap();
    let mut expected = output(domain);

    for workers in [1, 2, 5, 12] {
        let limits = ParallelReducedV2Force::preflight(domain, samples, workers).unwrap();
        let cap = limits.storage_bytes + 17;
        let mut parallel = ParallelReducedV2Force::new(domain, samples, workers, cap).unwrap();
        assert_eq!(
            parallel.identity(),
            nsbu_benchmarks::provider::parallel_reduced::ParallelReducedIdentity {
                retained: domain.layout(),
                sampled: samples,
                workers,
                cap_bytes: cap,
            }
        );
        for tick in [0, 1, 4, 2, 1] {
            let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
            let serial_work = serial
                .evaluate(
                    clock,
                    serial_limits,
                    expected.each_mut().map(Vec::as_mut_slice),
                )
                .unwrap();
            let mut actual = output(domain);
            let parallel_work = parallel
                .evaluate(clock, limits, actual.each_mut().map(Vec::as_mut_slice))
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(parallel_work.work_units, serial_work.work_units);
            assert_eq!(
                parallel_work.scalar_transforms,
                serial_work.scalar_transforms
            );
            assert_eq!(
                parallel.last_root_iterations(),
                serial.last_root_iterations()
            );
            for component in &actual {
                assert!(component.iter().all(|value| value.is_finite()));
                validate_spectrum(domain.layout(), component, 1e-12).unwrap();
            }
        }
    }
}

#[test]
fn admission_and_request_refusals_preserve_output() {
    let domain = domain();
    let samples = Layout::new([6; 3]).unwrap();
    assert!(ParallelReducedV2Force::preflight(domain, samples, 0).is_err());
    assert!(ParallelReducedV2Force::preflight(domain, samples, 7).is_err());
    let limits = ParallelReducedV2Force::preflight(domain, samples, 2).unwrap();
    assert!(matches!(
        ParallelReducedV2Force::new(domain, samples, 2, limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut provider =
        ParallelReducedV2Force::new(domain, samples, 2, limits.storage_bytes).unwrap();
    let mut values: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(3.0, -2.0); domain.layout().half_len()]);
    let prior = values.clone();
    let mut bad = limits;
    bad.work_units -= 1;
    assert!(provider
        .evaluate(
            TickClock::from_rest(-10, 8).unwrap(),
            bad,
            values.each_mut().map(Vec::as_mut_slice),
        )
        .is_err());
    assert_eq!(values, prior);
    let [a, b, c] = values.each_mut();
    assert!(provider
        .evaluate(
            TickClock::from_rest(-10, 8).unwrap(),
            limits,
            [&mut a[..1], b, c],
        )
        .is_err());
    assert_eq!(values, prior);
}

#[test]
fn shared_avx_catalog_preserves_reduced_force_and_refuses_underbudget() {
    use nsbu_benchmarks::provider::parallel::ParallelV2Force;
    use nsbu_solver::spectral::{FftBackend, FftCatalog};
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = domain();
    let samples = Layout::new([6; 3]).unwrap();
    let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
    assert!(ParallelV2Force::preflight_with_fft_backend(domain, samples, 2, backend).is_ok());
    let limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, 2, backend).unwrap();
    assert!(matches!(
        ParallelReducedV2Force::new_with_catalog(
            domain,
            samples,
            2,
            &catalog,
            limits.storage_bytes - 1,
        ),
        Err(SolverError::ResourceLimit)
    ));
    let mut parallel = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        2,
        &catalog,
        limits.storage_bytes,
    )
    .unwrap();
    let serial_limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, 1, backend).unwrap();
    let mut serial = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        1,
        &catalog,
        serial_limits.storage_bytes,
    )
    .unwrap();
    let clock = TickClock::restore(-10, 8, 2, 6).unwrap();
    let mut actual = output(domain);
    let mut expected = output(domain);
    parallel
        .evaluate(clock, limits, actual.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    serial
        .evaluate(
            clock,
            serial_limits,
            expected.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    for (a, b) in actual.iter().flatten().zip(expected.iter().flatten()) {
        assert_eq!(
            (a.re.to_bits(), a.im.to_bits()),
            (b.re.to_bits(), b.im.to_bits())
        );
    }
}
