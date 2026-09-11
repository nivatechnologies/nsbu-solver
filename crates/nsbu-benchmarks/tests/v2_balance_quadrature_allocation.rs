//! Isolated admission, construction and steady allocation audit for exact-v2 balances.
mod v2_family_support;
use nsbu_benchmarks::v2_experiment::{
    probes::{
        balances::{
            quadrature::{V2BalanceQuadrature, V2QuadraturePlan},
            V2BalancePlan,
        },
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks, settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn main() {
    let accepted = clocks();
    let coarse = [0, 64, 128].map(clock);
    let middle = [0, 32, 64, 96, 128].map(clock);
    let fine = [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes_plan = ProbePlan::new(
        family,
        TestedTimes::new(&fine, fine.len()).unwrap(),
        fine.len(),
        CAP,
    )
    .unwrap();
    let balance = V2BalancePlan::new(probes_plan, fine.len(), CAP).unwrap();
    let sets =
        [&coarse[..], &middle[..], &fine[..]].map(|set| TestedTimes::new(set, set.len()).unwrap());
    let plan = V2QuadraturePlan::new(balance, sets, 10_000, CAP).unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut probes = ProbeFamily::new(probes_plan).unwrap();
    let mut quadrature = V2BalanceQuadrature::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    probes.advance().unwrap();
    let steady = Region::new(GLOBAL);
    quadrature.measure(&probes).unwrap();
    let spent = steady.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    let work = quadrature.charged_work();
    println!(
        "v2 balance quadrature construction_bytes={} declared_bytes={} provider_work={} transforms={} steady_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        work.children.work_units,
        work.children.scalar_transforms,
    );
}
