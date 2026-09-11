//! Isolated aggregate admission, construction and steady-event allocation audit.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    diagnostic::{DiagnosticDriver, DiagnosticPlan, DiagnosticSettings},
    probes::ProbePlan,
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
    let manifest = [0, 7, 63, 64, 95, 127, 128]
        .map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let residual = [manifest[1], manifest[2], manifest[4], manifest[5]];
    let policy = DiagnosticSettings {
        physical_samples: Layout::new([12; 3]).unwrap(),
        pressure_samples: Layout::new([24; 3]).unwrap(),
        reference_samples: Layout::new([12; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
    };
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(
        settings(1e-5),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    let plan = DiagnosticPlan::new(family, probes, &residual, policy, CAP).unwrap();
    assert!(DiagnosticPlan::new(
        family,
        probes,
        &residual,
        policy,
        plan.bounds().joint_storage_bytes - 1,
    )
    .is_err());
    let admitted = admission.change();
    assert_eq!((admitted.allocations, admitted.reallocations), (0, 0));

    let construction = Region::new(GLOBAL);
    let mut driver = DiagnosticDriver::new(plan).unwrap();
    let built = construction.change();
    assert!(built.bytes_allocated <= plan.bounds().joint_storage_bytes);
    let execution = Region::new(GLOBAL);
    while driver.advance().unwrap().is_some() {}
    let spent = execution.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    assert_eq!(driver.reports().len(), manifest.len());
    assert_eq!(driver.charged_work(), plan.bounds().work);
    println!(
        "v2 diagnostic admission=0 construction_bytes={} joint_bytes={} events={} execution_allocations=0",
        built.bytes_allocated,
        plan.bounds().joint_storage_bytes,
        driver.reports().len(),
    );
}
