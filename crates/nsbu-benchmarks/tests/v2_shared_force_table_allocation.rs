//! Construction-bound and zero-steady-allocation audit for immutable table copies.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::shared_force::{SharedForceClock, SharedForceTable, SharedForceTablePlan},
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    Complex64,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let domains = [4, 8, 12].map(|n| Domain::new([n; 3], [1.0; 3], 1.0).unwrap());
    let settings = ForceSettings {
        samples: Layout::new([16; 3]).unwrap(),
        workers: 0,
    };
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let manifest = [SharedForceClock::new(clock, [1; 3]).unwrap()];
    let plan =
        SharedForceTablePlan::new(settings, domains, &manifest, 3, 256 * 1024 * 1024).unwrap();
    let (mut table, constructed) = {
        let before = Region::new(&INSTRUMENTED_SYSTEM);
        let table = SharedForceTable::new(plan).unwrap();
        (table, before.change())
    };
    assert!(constructed.bytes_allocated <= plan.bounds().construction_peak_bytes);
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domains[2].layout().half_len()]);
    let steady = Region::new(&INSTRUMENTED_SYSTEM);
    table
        .copy(
            plan.binding(2).unwrap(),
            clock,
            output.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let change = steady.change();
    assert_eq!(change.allocations, 0);
    assert_eq!(change.reallocations, 0);
    assert_eq!(change.deallocations, 0);
    eprintln!(
        "shared force construction_bytes={} declared_peak={} steady_allocations={}",
        constructed.bytes_allocated,
        plan.bounds().construction_peak_bytes,
        change.allocations
    );
}
