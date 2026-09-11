//! Complete optional-provider outputs against independent DFT and high-precision fixtures.
mod fixture_support;
mod reduced_provider_oracle;
use nsbu_benchmarks::{
    provider::{reduced::ReducedV2Force, V2Force},
    time::BenchmarkTime,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
fn domain(n: usize) -> Domain {
    Domain::new([n; 3], [1.0; 3], 1.0).unwrap()
}
fn clock(tick: u128) -> TickClock {
    TickClock::restore(-10, 8, tick, 8 - tick).unwrap()
}
fn outputs(layout: Layout) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(17.0, -3.0); layout.half_len()])
}
fn words(values: &[Vec<Complex64>; 3]) -> Vec<(u64, u64)> {
    values
        .iter()
        .flatten()
        .map(|z| (z.re.to_bits(), z.im.to_bits()))
        .collect()
}
#[test]
fn complete_spectra_match_uncached_direct_sums_and_original_provider_on_three_layouts() {
    for (n, shape) in [(4, [4, 4, 4]), (4, [6, 8, 12]), (8, [8, 12, 12])] {
        let domain = domain(n);
        let sampled = Layout::new(shape).unwrap();
        let limits = ReducedV2Force::preflight(domain, sampled).unwrap();
        let mut reduced = ReducedV2Force::new(domain, sampled, limits.storage_bytes).unwrap();
        let original_limits = V2Force::preflight(domain, sampled).unwrap();
        let mut original = V2Force::new(domain, sampled, original_limits.storage_bytes).unwrap();
        let mut actual = outputs(domain.layout());
        let mut previous = Vec::new();
        let mut baseline = outputs(domain.layout());
        for tick in [1, 4, 2, 1, 0] {
            let time = BenchmarkTime::new(clock(tick)).unwrap();
            let work = reduced
                .evaluate(
                    clock(tick),
                    limits,
                    actual.each_mut().map(Vec::as_mut_slice),
                )
                .unwrap();
            let (samples, planes, uncached) = reduced_provider_oracle::samples(sampled, time);
            let expected = reduced_provider_oracle::dft(domain.layout(), sampled, &samples);
            let error = reduced_provider_oracle::compare(&actual, &expected, 5e-12);
            let original_work = original
                .evaluate(
                    clock(tick),
                    original_limits,
                    baseline.each_mut().map(Vec::as_mut_slice),
                )
                .unwrap();
            let alternative_error = reduced_provider_oracle::compare(&actual, &baseline, 5e-12);
            assert_eq!(work.work_units, sampled.real_len() + planes);
            assert_eq!(work.work_units, original_work.work_units);
            assert_eq!(work.scalar_transforms, 3);
            assert_eq!(reduced.last_root_iterations(), planes);
            assert!(work.work_units <= limits.work_units);
            for component in &actual {
                nsbu_solver::domain::validate_spectrum(domain.layout(), component, 1e-12).unwrap();
            }
            if tick == 0 {
                assert_eq!(planes, 0);
                assert!(actual.iter().flatten().all(|z| z.l1_norm() == 0.0));
            } else {
                assert!(planes > 0 && planes < uncached);
            }
            if tick == 1 {
                let now = words(&actual);
                if !previous.is_empty() {
                    assert_eq!(now, previous);
                }
                previous = now;
            }
            println!("n={n} m={shape:?} tick={tick} work={} plane_iterations={planes} uncached_iterations={uncached} dft_scaled_error={error:e} cartesian_scaled_error={alternative_error:e}",work.work_units);
        }
    }
}

#[test]
fn independent_80_120_digit_n4_force_fixture_remains_unprojected() {
    let domain = domain(4);
    let limits = ReducedV2Force::preflight(domain, domain.layout()).unwrap();
    let mut provider = ReducedV2Force::new(domain, domain.layout(), limits.storage_bytes).unwrap();
    let mut output = outputs(domain.layout());
    provider
        .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let fixture = include_str!("fixtures/force-n4.tsv");
    assert_eq!(fixture.lines().count(), 18);
    fixture_support::compare(domain.layout(), &output, fixture, 5e-12);
    // Raw force has a longitudinal component; the provider must not project pressure away.
    let index = (4) * (4 / 2 + 1);
    assert!(output[0][index].l1_norm() > 1e-6);
}

#[test]
fn complete_admission_limits_and_request_refusals_precede_output_changes() {
    let domain = domain(4);
    let sampled = Layout::new([6, 8, 12]).unwrap();
    let limits = ReducedV2Force::preflight(domain, sampled).unwrap();
    assert_eq!(
        (
            limits.work_units,
            limits.scalar_transforms,
            limits.remaining_divisor
        ),
        (sampled.real_len() * 129, 3, 20)
    );
    for bad in [
        Domain::new([4; 3], [2.0; 3], 1.0).unwrap(),
        Domain::new([4; 3], [1.0; 3], 2.0).unwrap(),
    ] {
        assert_eq!(
            ReducedV2Force::preflight(bad, sampled).unwrap_err(),
            SolverError::InvalidDomain
        );
    }
    for dimensions in [[2; 3], [10; 3]] {
        assert_eq!(
            ReducedV2Force::preflight(domain, Layout::new(dimensions).unwrap()).unwrap_err(),
            SolverError::InvalidDomain
        );
    }
    // One complex buffer fits the Layout bound, while aggregate provider buffers overflow.
    let long = (isize::MAX as usize / 256 - 2) * 2;
    let huge = Layout::new([4, 4, long]).unwrap();
    assert_eq!(
        ReducedV2Force::preflight(domain, huge).unwrap_err(),
        SolverError::SizeOverflow
    );
    assert_eq!(
        ReducedV2Force::new(domain, sampled, limits.storage_bytes - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let mut provider = ReducedV2Force::new(domain, sampled, limits.storage_bytes).unwrap();
    assert_eq!(provider.limits(), Some(limits));
    let mut output = outputs(domain.layout());
    let before = words(&output);
    let mut bad = limits;
    bad.work_units -= 1;
    assert_eq!(
        provider
            .evaluate(clock(1), bad, output.each_mut().map(Vec::as_mut_slice))
            .unwrap_err(),
        SolverError::ProviderBudgetExceeded
    );
    assert_eq!(words(&output), before);
    let [a, b, c] = output.each_mut();
    assert_eq!(
        provider
            .evaluate(clock(1), limits, [a, b, &mut c[..1]])
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    assert_eq!(words(&output), before);
    assert_eq!(
        provider
            .evaluate(
                TickClock::from_rest(-9, 8).unwrap(),
                limits,
                output.each_mut().map(Vec::as_mut_slice)
            )
            .unwrap_err(),
        SolverError::InvalidClock
    );
    assert_eq!(words(&output), before);
    assert_eq!(provider.last_root_iterations(), 0);
    provider
        .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let saved = words(&output);
    let iterations = provider.last_root_iterations();
    assert!(iterations > 0);
    assert!(provider
        .evaluate(clock(2), bad, output.each_mut().map(Vec::as_mut_slice))
        .is_err());
    assert_eq!(words(&output), saved);
    assert_eq!(provider.last_root_iterations(), iterations);
}
