//! Isolated allocation check for admitted regional analytical tracking.
use nsbu_benchmarks::v2_experiment::{
    reference::{
        regional::{RegionalTrackingPlan, RegionalTrackingWorkspace},
        ReferenceTrackingPlan,
    },
    FamilyPlan, V2Family,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

mod v2_family_support;

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = v2_family_support::clocks();
    let family_plan = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let admission = Region::new(GLOBAL);
    let tracking = ReferenceTrackingPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [1e-8, 1e-7, 1e-6, 1e-7],
        4,
        v2_family_support::CAP,
    )
    .unwrap();
    let plan = RegionalTrackingPlan::new(tracking, 128, v2_family_support::CAP).unwrap();
    assert!(
        RegionalTrackingPlan::new(tracking, 128, plan.bounds().joint_storage_bytes - 1,).is_err()
    );
    let stats = admission.change();
    assert_eq!((stats.allocations, stats.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut regional = RegionalTrackingWorkspace::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    let execution = Region::new(GLOBAL);
    assert!(regional.measure(&family).is_err());
    for clock in clocks {
        family.advance().unwrap().unwrap();
        assert_eq!(regional.measure(&family).unwrap().clock(), clock);
    }
    let stats = execution.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    assert_eq!(regional.tracking_work(), plan.bounds().tracking_work);
    assert_eq!(regional.regional_work(), plan.bounds().regional_work);
    println!(
        "v2 regional tracking admission=0 construction_bytes={} joint_bytes={} reports=3 refusal=1 execution_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes
    );
}
