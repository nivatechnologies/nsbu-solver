//! Complete parallel/serial coefficient equality, exact actual work and refusal contracts.
use nsbu_benchmarks::provider::{parallel::ParallelV2Force, V2Force};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}
fn values() -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(23.0, -7.0); domain().layout().half_len()])
}
fn bits(value: &[Vec<Complex64>; 3]) -> Vec<(u64, u64)> {
    value
        .iter()
        .flatten()
        .map(|v| (v.re.to_bits(), v.im.to_bits()))
        .collect()
}
#[test]
fn complete_fields_and_work_match_for_all_partition_shapes_and_nonmonotone_times() {
    let samples = Layout::new([6, 8, 12]).unwrap();
    let serial_limits = V2Force::preflight(domain(), samples).unwrap();
    let mut serial = V2Force::new(domain(), samples, serial_limits.storage_bytes).unwrap();
    let mut reference = values();
    for workers in [1, 2, 5, 12] {
        let limits = ParallelV2Force::preflight(domain(), samples, workers).unwrap();
        assert_eq!(limits.work_units, serial_limits.work_units);
        assert_eq!(limits.scalar_transforms, serial_limits.scalar_transforms);
        let mut parallel =
            ParallelV2Force::new(domain(), samples, workers, limits.storage_bytes).unwrap();
        assert_eq!(parallel.limits(), Some(limits));
        let mut actual = values();
        for tick in [0, 1, 4, 2, 1, 0] {
            let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
            let expected = serial
                .evaluate(
                    clock,
                    serial_limits,
                    reference.each_mut().map(Vec::as_mut_slice),
                )
                .unwrap();
            let report = parallel
                .evaluate(clock, limits, actual.each_mut().map(Vec::as_mut_slice))
                .unwrap();
            assert_eq!(bits(&actual), bits(&reference));
            assert_eq!(report.work_units, expected.work_units);
            assert_eq!(report.scalar_transforms, 3);
            assert_eq!(
                parallel.last_root_iterations(),
                serial.last_root_iterations()
            );
            assert!(!parallel.is_terminated());
        }
    }
}
#[test]
fn admission_and_request_refusals_preserve_outputs_and_allow_a_valid_request() {
    let samples = Layout::new([4; 3]).unwrap();
    for workers in [0, 5, 129, usize::MAX] {
        assert!(ParallelV2Force::preflight(domain(), samples, workers).is_err());
    }
    assert!(
        ParallelV2Force::preflight(Domain::new([4; 3], [1.0; 3], 2.0).unwrap(), samples, 2)
            .is_err()
    );
    let limits = ParallelV2Force::preflight(domain(), samples, 2).unwrap();
    assert!(matches!(
        ParallelV2Force::new(domain(), samples, 2, limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut parallel = ParallelV2Force::new(domain(), samples, 2, limits.storage_bytes).unwrap();
    let mut output = values();
    let original = bits(&output);
    let clock = TickClock::from_rest(-10, 8).unwrap();
    let mut bad = limits;
    bad.work_units -= 1;
    assert_eq!(
        parallel
            .evaluate(clock, bad, output.each_mut().map(Vec::as_mut_slice))
            .unwrap_err(),
        SolverError::ProviderBudgetExceeded
    );
    assert_eq!(
        parallel
            .evaluate(
                TickClock::from_rest(-9, 8).unwrap(),
                limits,
                output.each_mut().map(Vec::as_mut_slice)
            )
            .unwrap_err(),
        SolverError::InvalidClock
    );
    let [a, b, c] = output.each_mut();
    assert_eq!(
        parallel
            .evaluate(clock, limits, [&mut a[..1], b, c])
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    assert_eq!(bits(&output), original);
    assert!(!parallel.is_terminated());
    parallel
        .evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    assert_eq!(parallel.last_root_iterations(), 0);
    assert!(output
        .iter()
        .flatten()
        .all(|v| *v == Complex64::new(0.0, 0.0)));
}
