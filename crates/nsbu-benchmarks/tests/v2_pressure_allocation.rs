//! Isolated allocator audit for pressure exact-v2 planning, construction and measurements.
use nsbu_benchmarks::v2_experiment::{
    pressure::{PressureFamilyPlan, PressureFamilyWorkspace},
    FamilyPlan, V2Family,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

mod v2_family_support;

fn main() {
    let clocks = v2_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let family_plan = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        times,
        v2_family_support::CAP,
    )
    .unwrap();
    let admission = Region::new(GLOBAL);
    let plan = PressureFamilyPlan::new(
        family_plan,
        Layout::new([24; 3]).unwrap(),
        [1e-8, 1e-7],
        4,
        v2_family_support::CAP,
    )
    .unwrap();
    let rejected = PressureFamilyPlan::new(
        family_plan,
        Layout::new([24; 3]).unwrap(),
        [1.0; 2],
        4,
        plan.bounds().joint_storage_bytes - 1,
    );
    assert!(rejected.is_err());
    let planned = admission.change();
    assert_eq!((planned.allocations, planned.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut pressure = PressureFamilyWorkspace::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);

    let execution = Region::new(GLOBAL);
    assert!(pressure.measure(&family).is_err());
    for expected in clocks {
        family.advance().unwrap().unwrap();
        assert_eq!(pressure.measure(&family).unwrap().clock(), expected);
    }
    assert!(pressure.measure(&family).is_err());
    let steady = execution.change();
    assert_eq!(
        (
            steady.allocations,
            steady.deallocations,
            steady.reallocations
        ),
        (0, 0, 0)
    );
    assert_eq!(pressure.charged_work(), plan.bounds().work);
    println!("v2 pressure admission_allocations=0 construction_bytes={} steady_allocations=0 reports=3 refusals=2 declared_joint_bytes={}", built.bytes_allocated, plan.bounds().joint_storage_bytes);
}
