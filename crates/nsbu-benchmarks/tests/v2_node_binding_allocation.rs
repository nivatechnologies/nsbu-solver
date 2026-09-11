//! Isolated allocation check for accepted-node binding admission and execution.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    binding::{NodeBindingPlan, NodeBindingWorkspace},
    probes::{ProbeFamily, ProbePlan},
    FamilyPlan, V2Family,
};
use nsbu_solver::verification::times::TestedTimes;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks, settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let clocks = clocks();
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        clocks.len(),
        CAP,
    )
    .unwrap();
    let plan = NodeBindingPlan::new(family, probes, 4, CAP).unwrap();
    assert!(
        NodeBindingPlan::new(family, probes, 4, plan.bounds().joint_storage_bytes - 1,).is_err()
    );
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut ordinary = V2Family::new(family).unwrap();
    let mut probes = ProbeFamily::new(probes).unwrap();
    let mut binding = NodeBindingWorkspace::new(plan);
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    let execution = Region::new(GLOBAL);
    assert!(binding.measure(&ordinary, &probes).is_err());
    for clock in clocks {
        ordinary.advance().unwrap();
        probes.advance().unwrap();
        assert_eq!(binding.measure(&ordinary, &probes).unwrap().clock(), clock);
    }
    let spent = execution.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    assert_eq!(binding.charged_work(), plan.bounds().work);
    println!(
        "v2 node binding admission=0 construction_bytes={} joint_bytes={} reports=3 refusal=1 execution_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
    );
}
