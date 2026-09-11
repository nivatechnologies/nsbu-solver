//! Isolated allocation audit of joint exact-v2 admission and successful family samples.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyPlan, FamilySettings, V2Family},
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let settings = FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [1e-5; 2],
        },
        advective_limit: 0.3,
    };
    let admission = Region::new(GLOBAL);
    let plan = FamilyPlan::new(settings, times, 1 << 27).unwrap();
    assert!(FamilyPlan::new(settings, times, plan.bounds().storage_bytes - 1).is_err());
    let stats = admission.change();
    assert_eq!((stats.allocations, stats.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = V2Family::new(plan).unwrap();
    let actual_heap = construction.change().bytes_allocated;
    assert!(actual_heap <= plan.bounds().storage_bytes);
    let execution = Region::new(GLOBAL);
    for expected in clocks {
        let sample = family.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), expected);
        for index in 0..6 {
            assert_eq!(family.branch(index).unwrap().state().clock(), expected);
        }
    }
    assert!(family.advance().unwrap().is_none());
    assert!(family.advance().unwrap().is_none());
    let stats = execution.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    assert!(family
        .branch(2)
        .unwrap()
        .state()
        .component(0)
        .unwrap()
        .iter()
        .any(|z| z.re != 0.0 || z.im != 0.0));
    println!(
        "exact-v2 family admission=0 construction_bytes={actual_heap} declared_bytes={} sampled_times=3 steady_allocations=0",
        plan.bounds().storage_bytes
    );
}
