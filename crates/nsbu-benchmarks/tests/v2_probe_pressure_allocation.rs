//! Allocation audit for the reconstructed pressure consumer and its probe owner.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    probes::{
        pressure::{ProbePressurePlan, ProbePressureWorkspace},
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks, settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let accepted = clocks();
    let times =
        [0, 7, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let admission = Region::new(GLOBAL);
    let family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(&accepted, 3).unwrap(), CAP).unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(&times, 3).unwrap(), 3, CAP).unwrap();
    let plan = ProbePressurePlan::new(probes, Layout::new([24; 3]).unwrap(), [1e-8, 1e-7], 3, CAP)
        .unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut probes = ProbeFamily::new(probes).unwrap();
    let mut pressure = ProbePressureWorkspace::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    for _ in times {
        let raw = probes.advance().unwrap().unwrap();
        let steady = Region::new(GLOBAL);
        pressure.measure(&probes, raw).unwrap();
        let spent = steady.change();
        assert_eq!(
            (spent.allocations, spent.deallocations, spent.reallocations),
            (0, 0, 0)
        );
    }
    println!(
        "v2 probe pressure construction_bytes={} declared_bytes={} transforms={} visits={} steady_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        pressure.charged_work().scalar_transforms,
        pressure.charged_work().weighted_visits,
    );
}
