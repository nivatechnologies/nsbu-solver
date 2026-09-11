//! Isolated planning, construction and steady-work allocation audit for exact-v2 residuals.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    probes::{
        residuals::{ResidualFamily, ResidualFamilyPlan},
        ProbeFamily, ProbePlan,
    },
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
    let subset = [probes[1]];
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes_plan = ProbePlan::new(
        family,
        TestedTimes::new(&probes, probes.len()).unwrap(),
        probes.len(),
        CAP,
    )
    .unwrap();
    let plan = ResidualFamilyPlan::new(probes_plan, &subset, 1, CAP).unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut probes = ProbeFamily::new(probes_plan).unwrap();
    let mut residuals = ResidualFamily::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    probes.advance().unwrap();
    probes.advance().unwrap();
    let steady = Region::new(GLOBAL);
    residuals.measure(&probes).unwrap();
    let spent = steady.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    println!(
        "v2 residuals construction_bytes={} declared_bytes={} provider_work={} transforms={} coefficient_work={} steady_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        residuals.charged_work().residual.provider_work_units,
        residuals.charged_work().residual.scalar_transforms,
        residuals.charged_work().residual.coefficient_work_units,
    );
}
