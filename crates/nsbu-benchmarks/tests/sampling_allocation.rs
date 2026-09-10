//! Isolated heap instrumentation for joint admission and complete successful/refused sampling.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    sampling::{SamplingPlan, SamplingWorkspace},
    FamilyPlan, SmoothFamily,
};
use nsbu_solver::{domain::Layout, verification::times::TestedTimes};
use stats_alloc::Region;
#[global_allocator]
static GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = smooth_family_support::clocks();
    let layouts = [24, 32, 36].map(|n| Layout::new([n; 3]).unwrap());
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        TestedTimes::new(&clocks, 3).unwrap(),
        smooth_family_support::CAP,
    )
    .unwrap();
    let region = Region::new(GLOBAL);
    let plan = SamplingPlan::new(
        family_plan,
        layouts,
        [1e-8; 6],
        5,
        smooth_family_support::CAP,
    )
    .unwrap();
    assert!(SamplingPlan::new(
        family_plan,
        layouts,
        [1e-8; 6],
        5,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let admitted = region.change();
    assert_eq!(
        (
            admitted.allocations,
            admitted.deallocations,
            admitted.reallocations
        ),
        (0, 0, 0)
    );
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let construction = Region::new(GLOBAL);
    let mut consumer = SamplingWorkspace::new(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.bounds().storage_bytes);
    let execution = Region::new(GLOBAL);
    assert!(consumer.measure(&family).is_err());
    for expected in clocks {
        family.advance().unwrap().unwrap();
        let result = consumer.measure(&family).unwrap();
        assert_eq!(result.clock(), expected);
        for quantity in result.quantities() {
            assert!(quantity.changes(4).is_ok());
        }
    }
    assert!(consumer.measure(&family).is_err());
    assert!(consumer.measure(&family).is_err());
    let actual = execution.change();
    assert_eq!(
        (
            actual.allocations,
            actual.deallocations,
            actual.reallocations
        ),
        (0, 0, 0)
    );
    assert_eq!(consumer.charged_work(), plan.bounds().work);
}
