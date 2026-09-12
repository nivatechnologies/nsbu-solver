//! Dedicated allocation audit for the opt-in attempt-local exact-v2 force cache.
use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let settings = ForceSettings {
        samples: Layout::new([4; 3]).unwrap(),
        workers: 0,
    };
    let admission = Region::new(GLOBAL);
    let limits = settings.attempt_cache_limits(domain).unwrap();
    assert!(matches!(
        settings.build_attempt_cache(domain, limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let admission_change = admission.change();
    assert_eq!(
        (admission_change.allocations, admission_change.reallocations),
        (0, 0)
    );
    let construction = Region::new(GLOBAL);
    let mut force = settings
        .build_attempt_cache(domain, limits.storage_bytes)
        .unwrap();
    assert!(construction.change().bytes_allocated <= limits.storage_bytes);
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let stages = clock.stages(128).unwrap();
    let mut output =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    force.begin_attempt(clock, 128, limits).unwrap();
    let steady = Region::new(GLOBAL);
    for stage in stages.into_iter().chain(stages) {
        force
            .evaluate(stage, limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap();
    }
    let change = steady.change();
    assert_eq!(
        (
            change.allocations,
            change.deallocations,
            change.reallocations
        ),
        (0, 0, 0)
    );
    assert_eq!(force.work().provider_evaluations, 5);
}
