//! Isolated allocator check includes first call, root/jet work, flat branches and refusals.
use nsbu_benchmarks::{reduced_force, time::BenchmarkTime};
use nsbu_solver::domain::TickClock;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
fn main() {
    let region = Region::new(GLOBAL);
    for tick in [1, 4, 2, 1, 0] {
        let time = BenchmarkTime::new(TickClock::restore(-10, 8, tick, 8 - tick).unwrap()).unwrap();
        for point in [
            [0.0; 3],
            [0.125, -0.0625, 0.125],
            [0.35, 0.0, 0.05],
            [0.5; 3],
        ] {
            let sample = reduced_force::evaluate(point, time).unwrap();
            assert!(sample.force.iter().all(|v| v.is_finite()));
        }
        assert!(reduced_force::evaluate([f64::NAN, 0.0, 0.0], time).is_err());
    }
    let measured = region.change();
    assert_eq!(
        (
            measured.allocations,
            measured.deallocations,
            measured.reallocations
        ),
        (0, 0, 0)
    );
    println!("reduced force first/repeated/nonmonotone/flat/refused calls: allocations=0 deallocations=0 reallocations=0; scalar/root/jet storage is fixed on the stack");
}
