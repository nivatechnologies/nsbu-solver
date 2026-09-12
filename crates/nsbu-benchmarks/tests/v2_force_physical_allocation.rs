//! Isolated allocation audit for physical force-grid planning and measurement.
mod v2_force_family_support;

use nsbu_benchmarks::v2_force_experiment::{
    physical::{ForcePhysicalPlan, ForcePhysicalWorkspace},
    ForceFamily, ForceFamilyPlan,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = v2_force_family_support::clocks();
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let family_plan = ForceFamilyPlan::new(
        v2_force_family_support::settings(1e-5),
        times,
        v2_force_family_support::CAP,
    )
    .unwrap();
    let admission = Region::new(GLOBAL);
    let plan = ForcePhysicalPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [1e-8, 1e-7, 1e-6, 1e-7],
        clocks.len(),
        v2_force_family_support::CAP,
    )
    .unwrap();
    assert!(ForcePhysicalPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [1.0; 4],
        clocks.len(),
        plan.bounds().joint_storage_bytes - 1,
    )
    .is_err());
    let planned = admission.change();
    assert_eq!((planned.allocations, planned.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = ForceFamily::from_rest(family_plan).unwrap();
    let mut physical = ForcePhysicalWorkspace::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);

    let execution = Region::new(GLOBAL);
    for clock in clocks {
        let raw = family.advance().unwrap().unwrap();
        assert_eq!(physical.measure(&family, raw).unwrap().clock(), clock);
    }
    let steady = execution.change();
    assert_eq!(
        (
            steady.allocations,
            steady.deallocations,
            steady.reallocations
        ),
        (0, 0, 0)
    );
    assert_eq!(physical.charged_work(), plan.bounds().work);
    println!(
        "force physical construction_bytes={} joint_bytes={} steady_allocations=0 reports=3",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
    );
}
