//! Provider heap accounting and nonmonotone stage requests in an isolated process.
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64,
};
use stats_alloc::Region;

#[global_allocator]
static GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;

fn main() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let sampled = Layout::new([6; 3]).unwrap();
    let limits = V2Force::preflight(domain, sampled).unwrap();
    let planning = Region::new(GLOBAL);
    let mut provider = V2Force::new(domain, sampled, limits.storage_bytes).unwrap();
    let allocated = planning.change();
    assert_eq!(allocated.allocations, 11);
    assert_eq!(allocated.reallocations, 0);
    let object_bytes = std::mem::size_of::<V2Force>()
        + std::mem::size_of::<nsbu_solver::spectral::FftPlan>()
        + std::mem::size_of::<nsbu_solver::spectral::FftWorkspace>();
    assert_eq!(
        limits.storage_bytes,
        allocated.bytes_allocated + object_bytes + 11 * 64
    );
    let mut buffers: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut first: [Vec<Complex64>; 3] = buffers.clone();
    let region = Region::new(GLOBAL);
    for elapsed in [1, 4, 2, 3, 0, 1] {
        let clock = TickClock::restore(-10, 8, elapsed, 8 - elapsed).unwrap();
        let [a, b, c] = &mut buffers;
        let work = provider.evaluate(clock, limits, [a, b, c]).unwrap();
        assert!(work.work_units <= limits.work_units);
        for (saved, current) in first.iter_mut().zip(&buffers) {
            if elapsed == 1 {
                if saved.iter().all(|v| *v == Complex64::new(0.0, 0.0)) {
                    saved.copy_from_slice(current);
                }
                assert_eq!(saved, current);
            }
        }
    }
    let actual = region.change();
    assert_eq!(
        (
            actual.allocations,
            actual.deallocations,
            actual.reallocations
        ),
        (0, 0, 0)
    );
}
