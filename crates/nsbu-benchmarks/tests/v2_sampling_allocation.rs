//! Isolated allocation audit for nested exact-v2 physical sampling.
use nsbu_benchmarks::v2_experiment::{
    sampling::{SamplingPlan, SamplingWorkspace},
    FamilyPlan, V2Family,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

mod v2_family_support;

fn main() {
    let clocks = v2_family_support::clocks();
    let family_plan = FamilyPlan::new(
        v2_family_support::settings(1e-5),
        TestedTimes::new(&clocks, 3).unwrap(),
        v2_family_support::CAP,
    )
    .unwrap();
    let admission = Region::new(GLOBAL);
    let layouts = [12, 24, 48].map(|n| Layout::new([n; 3]).unwrap());
    let plan = SamplingPlan::new(
        family_plan,
        layouts,
        [1e-8, 1e-7, 1e-6, 1e-7],
        4,
        v2_family_support::CAP,
    )
    .unwrap();
    assert!(SamplingPlan::new(
        family_plan,
        layouts,
        [1.0; 4],
        4,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut family = V2Family::new(family_plan).unwrap();
    let mut sampling = SamplingWorkspace::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);

    let execution = Region::new(GLOBAL);
    assert!(sampling.measure(&family).is_err());
    for clock in clocks {
        family.advance().unwrap();
        assert_eq!(sampling.measure(&family).unwrap().clock(), clock);
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
    assert_eq!(sampling.charged_work(), plan.bounds().work);
    println!("v2 nested sampling admission_allocations=0 construction_bytes={} steady_allocations=0 reports=3 refusals=1 declared_joint_bytes={}",built.bytes_allocated,plan.bounds().joint_storage_bytes);
}
