//! Full probe-owner admission and repeated sampling require no heap activity after construction.
use nsbu_benchmarks::smooth_experiment::{
    probes::{ProbeFamily, ProbePlan},
    FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::Region;
pub fn check() {
    let accepted = super::smooth_family_support::clocks();
    let clocks =
        [0, 7, 31, 63, 64, 95, 127, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let cap = super::smooth_family_support::CAP;
    let family = FamilyPlan::new(
        super::smooth_family_support::settings(1e-2),
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let times = TestedTimes::new(&clocks, 8).unwrap();
    let planning = Region::new(super::GLOBAL);
    let plan = ProbePlan::new(family, times, 8, cap).unwrap();
    assert!(ProbePlan::new(family, times, 8, plan.bounds().joint_storage_bytes - 1).is_err());
    let measured = planning.change();
    assert_eq!(
        (
            measured.allocations,
            measured.deallocations,
            measured.reallocations
        ),
        (0, 0, 0)
    );
    let construction = Region::new(super::GLOBAL);
    let mut run = ProbeFamily::new(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let sampling = Region::new(super::GLOBAL);
    for clock in clocks {
        let sample = run.advance().unwrap().unwrap();
        assert_eq!(sample.clock(), clock);
        assert!(run.fields(2).is_some());
    }
    assert!(run.advance().unwrap().is_none());
    let measured = sampling.change();
    assert_eq!(
        (
            measured.allocations,
            measured.deallocations,
            measured.reallocations
        ),
        (0, 0, 0)
    );
}
