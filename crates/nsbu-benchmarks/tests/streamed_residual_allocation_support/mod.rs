//! Independent six-branch residual admission/measurement has no hidden post-construction heap work.
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        residuals::{ResidualFamily, ResidualFamilyPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::Region;
pub fn check() {
    let accepted = super::smooth_family_support::clocks();
    let times = [0, 7, 31, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let cap = super::smooth_family_support::CAP;
    let family = FamilyPlan::new(
        super::smooth_family_support::settings(1e-2),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let owner = ProbePlan::new(family, TestedTimes::new(&times, 4).unwrap(), 4, cap).unwrap();
    let admission = Region::new(super::GLOBAL);
    let plan = ResidualFamilyPlan::new(owner, &times[1..3], 3, cap).unwrap();
    assert!(ResidualFamilyPlan::new(
        owner,
        &times[1..3],
        3,
        plan.bounds().joint_storage_bytes - 1
    )
    .is_err());
    let stats = admission.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    let construction = Region::new(super::GLOBAL);
    let mut family = ProbeFamily::new(owner).unwrap();
    let mut residuals = ResidualFamily::new(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let sampling = Region::new(super::GLOBAL);
    assert!(residuals.measure(&family).is_err());
    while let Some(probe) = family.advance().unwrap() {
        if residuals.next_time() == Some(probe.clock()) {
            let report = residuals.measure(&family).unwrap();
            assert_eq!(report.clock(), probe.clock());
        }
    }
    assert!(residuals.measure(&family).is_err());
    let stats = sampling.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
}
