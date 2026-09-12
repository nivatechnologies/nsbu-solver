//! Isolated allocation accounting for the persistent reduced worker pool.
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6, 8, 12]).unwrap();
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let admission = Region::new(GLOBAL);
    let limits = ParallelReducedV2Force::preflight(domain, samples, 3).unwrap();
    assert!(matches!(
        ParallelReducedV2Force::new(domain, samples, 3, limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    assert!(ParallelReducedV2Force::preflight(domain, samples, 0).is_err());
    let refused = admission.change();
    assert_eq!(
        (
            refused.allocations,
            refused.deallocations,
            refused.reallocations
        ),
        (0, 0, 0)
    );
    let construction = Region::new(GLOBAL);
    let mut provider =
        ParallelReducedV2Force::new(domain, samples, 3, limits.storage_bytes).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated + 3 * 2 * 1024 * 1024 <= limits.storage_bytes);
    assert_eq!(built.reallocations, 0);
    let steady = Region::new(GLOBAL);
    for tick in [1, 4, 2, 1, 0] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
        let report = provider
            .evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        assert_eq!(
            report.work_units,
            samples.real_len() + provider.last_root_iterations()
        );
    }
    let mut bad = limits;
    bad.work_units -= 1;
    assert!(provider
        .evaluate(
            TickClock::from_rest(-10, 8).unwrap(),
            bad,
            output.each_mut().map(Vec::as_mut_slice),
        )
        .is_err());
    assert!(provider
        .evaluate(
            TickClock::from_rest(-9, 8).unwrap(),
            limits,
            output.each_mut().map(Vec::as_mut_slice),
        )
        .is_err());
    let measured = steady.change();
    assert_eq!(
        (
            measured.allocations,
            measured.deallocations,
            measured.reallocations
        ),
        (0, 0, 0)
    );
    println!(
        "parallel reduced constructor allocations={} bytes={} configured_stack_bytes={} total_reservation={}; first/repeated/refused requests allocate/deallocate/reallocate nothing",
        built.allocations,
        built.bytes_allocated,
        3 * 2 * 1024 * 1024,
        limits.storage_bytes
    );
}
