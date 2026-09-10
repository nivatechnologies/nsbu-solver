//! Joint admission and repeated full physical probes have an isolated allocator contract.
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        diagnostics::{ProbeDiagnostics, ProbeDiagnosticsPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
};
use stats_alloc::Region;
pub fn check() {
    let accepted = super::smooth_family_support::clocks();
    let times = [0, 7, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let cap = super::smooth_family_support::CAP;
    let family = FamilyPlan::new(
        super::smooth_family_support::settings(1e-2),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(&times, 3).unwrap(), 3, cap).unwrap();
    let samples = Layout::new([24; 3]).unwrap();
    let admission = Region::new(super::GLOBAL);
    let plan = ProbeDiagnosticsPlan::new(probes, samples, [1.0; 6], 4, cap).unwrap();
    assert!(ProbeDiagnosticsPlan::new(
        probes,
        samples,
        [1.0; 6],
        4,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let stats = admission.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    let allocation = Region::new(super::GLOBAL);
    let mut family = ProbeFamily::new(probes).unwrap();
    let mut diagnostics = ProbeDiagnostics::new(plan).unwrap();
    assert!(allocation.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let sampling = Region::new(super::GLOBAL);
    assert!(diagnostics.measure(&family).is_err());
    for clock in times {
        family.advance().unwrap();
        assert_eq!(diagnostics.measure(&family).unwrap().clock(), clock);
    }
    assert!(diagnostics.measure(&family).is_err());
    let stats = sampling.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
}
