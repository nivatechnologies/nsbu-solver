//! Force-provider admission, exact requests and independent DFT coefficients.
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};

fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}
fn outputs(layout: Layout) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()])
}

#[test]
fn admission_refuses_mismatched_case_grid_and_cap() {
    let layout = domain().layout();
    assert_eq!(
        V2Force::preflight(Domain::new([4; 3], [2.0; 3], 1.0).unwrap(), layout).unwrap_err(),
        SolverError::InvalidDomain
    );
    assert_eq!(
        V2Force::preflight(Domain::new([4; 3], [1.0; 3], 2.0).unwrap(), layout).unwrap_err(),
        SolverError::InvalidDomain
    );
    assert_eq!(
        V2Force::preflight(domain(), Layout::new([2; 3]).unwrap()).unwrap_err(),
        SolverError::InvalidDomain
    );
    assert_eq!(
        V2Force::new(domain(), layout, 0).unwrap_err(),
        SolverError::ResourceLimit
    );
    let limits = V2Force::preflight(domain(), layout).unwrap();
    assert_eq!(limits.work_units, 64 * 129);
    assert_eq!(limits.remaining_divisor, 20);
    assert_eq!(limits.scalar_transforms, 3);
    assert_eq!(
        V2Force::new(domain(), layout, limits.storage_bytes - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
}

#[test]
fn request_validation_precedes_output_changes() {
    let layout = domain().layout();
    let mut provider = V2Force::new(domain(), layout, 1 << 20).unwrap();
    let limits = provider.limits().unwrap();
    let mut bad_limits = limits;
    bad_limits.work_units -= 1;
    let clock = TickClock::from_rest(-10, 8).unwrap();
    let [mut a, mut b, mut c] = outputs(layout);
    assert_eq!(
        provider
            .evaluate(clock, bad_limits, [&mut a, &mut b, &mut c])
            .unwrap_err(),
        SolverError::ProviderBudgetExceeded
    );
    assert_eq!(
        provider
            .evaluate(clock, limits, [&mut a[..1], &mut b, &mut c])
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    assert_eq!(
        provider
            .evaluate(
                TickClock::from_rest(-9, 8).unwrap(),
                limits,
                [&mut a, &mut b, &mut c]
            )
            .unwrap_err(),
        SolverError::InvalidClock
    );
    let work = provider
        .evaluate(clock, limits, [&mut a, &mut b, &mut c])
        .unwrap();
    assert_eq!(work.work_units, 64);
    assert_eq!(work.scalar_transforms, 3);
    assert_eq!(provider.last_root_iterations(), 0);
    for value in a.iter().chain(&b).chain(&c) {
        assert_eq!(*value, Complex64::new(0.0, 0.0));
    }
}

#[test]
fn normalized_force_coefficients_match_independent_direct_dft() {
    let layout = domain().layout();
    let limits = V2Force::preflight(domain(), layout).unwrap();
    let mut provider = V2Force::new(domain(), layout, limits.storage_bytes).unwrap();
    let [mut a, mut b, mut c] = outputs(layout);
    let work = provider
        .evaluate(
            TickClock::restore(-10, 8, 1, 7).unwrap(),
            limits,
            [&mut a, &mut b, &mut c],
        )
        .unwrap();
    assert_eq!(work.work_units, 64 + provider.last_root_iterations());
    assert!(work.work_units <= limits.work_units);
    assert!(provider.last_root_iterations() > 0);
    let output = [a, b, c];
    for line in include_str!("fixtures/force-n4.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let mode = std::array::from_fn(|i| columns[i].parse::<isize>().unwrap());
        let (index, conjugate) = layout.locate(mode).unwrap();
        assert!(!conjugate);
        for component in 0..3 {
            let expected = Complex64::new(
                columns[3 + 2 * component].parse().unwrap(),
                columns[4 + 2 * component].parse().unwrap(),
            );
            assert!(
                (output[component][index] - expected).l1_norm()
                    < 2e-10 * (1.0 + expected.l1_norm())
            );
        }
    }
    for component in output {
        nsbu_solver::domain::validate_spectrum(layout, &component, 1e-12).unwrap();
    }
}
