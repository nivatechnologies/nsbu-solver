//! Isolated allocation audit for the exact-v2 reconstructed probe family.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    probes::{ProbeFamily, ProbePlan},
    FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks, settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let accepted = clocks();
    let probes =
        [0, 7, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let plan = ProbePlan::new(
        family,
        TestedTimes::new(&probes, probes.len()).unwrap(),
        probes.len(),
        CAP,
    )
    .unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));
    assert!(ProbePlan::new(
        family,
        TestedTimes::new(&probes, probes.len()).unwrap(),
        probes.len(),
        plan.bounds().joint_storage_bytes - 1,
    )
    .is_err());

    let construction = Region::new(GLOBAL);
    let mut probes = ProbeFamily::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    while probes.next_time().is_some() {
        let steady = Region::new(GLOBAL);
        probes.advance().unwrap().unwrap();
        let spent = steady.change();
        assert_eq!(
            (spent.allocations, spent.deallocations, spent.reallocations),
            (0, 0, 0)
        );
    }
    println!(
        "v2 probes construction_bytes={} declared_bytes={} attempts={} weighted_visits={} steady_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        probes.charged_work().attempts,
        probes.charged_work().weighted_visits,
    );
}
