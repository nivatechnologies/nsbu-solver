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
    smooth_admission_and_attempts();
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

fn smooth_admission_and_attempts() {
    use nsbu_benchmarks::smooth_run::{archive, SmoothPlan, SmoothRun};
    use nsbu_solver::{
        experiment::control::Configuration,
        integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let configuration = Configuration {
            limits: RunLimits {
                endpoint: 2048,
                step_ticks: 1024,
                maximum_attempts: 2,
            },
            method,
            tolerances: Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
        };
        let admission = Region::new(GLOBAL);
        let plan = SmoothPlan::from_rest(domain, clock, configuration, 2, 0.3, 1 << 23).unwrap();
        assert!(SmoothPlan::from_rest(domain, clock, configuration, 2, 0.3, 1).is_err());
        let allocation = admission.change();
        assert_eq!((allocation.allocations, allocation.reallocations), (0, 0));
        let construction = Region::new(GLOBAL);
        let mut run = SmoothRun::from_rest(
            domain,
            clock,
            configuration,
            2,
            0.3,
            plan.resources().total(),
        )
        .unwrap();
        assert!(construction.change().bytes_allocated <= plan.resources().total());
        let stepping = Region::new(GLOBAL);
        run.step().unwrap();
        let first_allocation = stepping.change();
        assert_eq!(
            (first_allocation.allocations, first_allocation.reallocations),
            (0, 0)
        );
        let mut bytes = vec![0; archive::encoded_len(&run).unwrap()];
        archive::write(&run, &mut bytes).unwrap();
        let imported = archive::read(&bytes, run.state().plan(), bytes.len(), 1 << 23).unwrap();
        let mut resumed = imported.continue_unverified(1 << 23).unwrap();
        let stepping = Region::new(GLOBAL);
        let resumed_outcome = resumed.step();
        assert_eq!(resumed_outcome, run.step());
        let allocation = stepping.change();
        assert_eq!(
            (
                allocation.allocations,
                allocation.reallocations,
                allocation.deallocations
            ),
            (0, 0, 0)
        );
        assert_eq!(run.history().controller().committed(), 2);
    }
}
