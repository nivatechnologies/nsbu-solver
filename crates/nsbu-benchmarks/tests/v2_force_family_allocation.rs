//! Isolated allocation audit for family admission, construction, evolution and refusal.
use nsbu_benchmarks::v2_force_experiment::{ForceFamily, ForceFamilyPlan};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

mod v2_force_family_support;
use v2_force_family_support::{settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap());
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let settings = nsbu_benchmarks::v2_force_experiment::ForceFamilySettings {
        endpoint: 128,
        ..settings(1e-5)
    };
    let admission = Region::new(GLOBAL);
    let plan = ForceFamilyPlan::new(settings, times, CAP).unwrap();
    assert!(ForceFamilyPlan::new(settings, times, plan.bounds().storage_bytes - 1).is_err());
    let stats = admission.change();
    assert_eq!((stats.allocations, stats.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = ForceFamily::from_rest(plan).unwrap();
    let actual = construction.change().bytes_allocated;
    assert!(actual <= plan.bounds().storage_bytes);
    let execution = Region::new(GLOBAL);
    for clock in clocks {
        assert_eq!(family.advance().unwrap().unwrap().clock(), clock);
    }
    assert!(family.advance().unwrap().is_none());
    assert!(family.advance().unwrap().is_none());
    let stats = execution.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    println!(
        "v2 force family admission=0 construction_bytes={actual} declared_bytes={} attempts={} steady_allocations=0",
        plan.bounds().storage_bytes,
        plan.bounds().attempts
    );
}
