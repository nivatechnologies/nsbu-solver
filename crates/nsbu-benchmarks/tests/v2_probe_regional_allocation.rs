//! Allocation audit for off-stage regional analytical tracking.
mod v2_family_support;
use nsbu_benchmarks::v2_experiment::{
    probes::{
        reference::{ProbeReferencePlan, ProbeReferenceWorkspace},
        regional::{ProbeRegionalPlan, ProbeRegionalWorkspace},
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks as accepted_clocks, settings, CAP};
#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;
fn main() {
    let accepted = accepted_clocks();
    let times =
        [0, 7, 63, 64, 95, 127, 128].map(|n| TickClock::restore(-20, 8192, n, 8192 - n).unwrap());
    let admission = Region::new(GLOBAL);
    let family =
        FamilyPlan::new(settings(1e-5), TestedTimes::new(&accepted, 3).unwrap(), CAP).unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(&times, 7).unwrap(), 7, CAP).unwrap();
    let reference = ProbeReferencePlan::new(
        probes,
        Layout::new([12; 3]).unwrap(),
        [1e-8, 1e-7, 1e-6, 1e-7],
        7,
        CAP,
    )
    .unwrap();
    let regional = ProbeRegionalPlan::new(reference, 128, CAP).unwrap();
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));
    let construction = Region::new(GLOBAL);
    let mut owner = ProbeFamily::new(probes).unwrap();
    let mut reference_owner = ProbeReferenceWorkspace::new(reference).unwrap();
    let mut regional_owner = ProbeRegionalWorkspace::new(regional).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= regional.bounds().joint_storage_bytes);
    for _ in times {
        let raw = owner.advance().unwrap().unwrap();
        let reference_sample = reference_owner.measure(&owner, raw).unwrap();
        let steady = Region::new(GLOBAL);
        regional_owner
            .measure(&owner, raw, reference_sample)
            .unwrap();
        let spent = steady.change();
        assert_eq!(
            (spent.allocations, spent.deallocations, spent.reallocations),
            (0, 0, 0)
        );
    }
    println!("probe regional construction_bytes={} joint_bytes={} transforms={} classifications={} steady_allocations=0",built.bytes_allocated,regional.bounds().joint_storage_bytes,regional_owner.tracking_work().scalar_transforms,regional_owner.regional_work().classifications);
}
