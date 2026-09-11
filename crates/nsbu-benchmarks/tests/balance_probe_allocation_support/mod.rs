//! Full simultaneous admission, construction and streamed quadrature allocation accounting.
use crate::{smooth_family_support, GLOBAL};
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        balances::{
            quadrature::{BalanceQuadrature, QuadraturePlan},
            BalanceProbePlan,
        },
        ProbeFamily, ProbePlan,
    },
    FamilyPlan,
};
use nsbu_solver::{domain::TickClock, verification::times::TestedTimes};
use stats_alloc::Region;
pub fn check() {
    let accepted = smooth_family_support::clocks();
    let middle = [0, 32, 64, 96, 128].map(clock);
    let fine = [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let sets =
        [&accepted[..], &middle[..], &fine[..]].map(|s| TestedTimes::new(s, s.len()).unwrap());
    let cap = smooth_family_support::CAP;
    let admission = Region::new(GLOBAL);
    let family = FamilyPlan::new(smooth_family_support::settings(1e-2), sets[0], cap).unwrap();
    let probes = ProbePlan::new(family, sets[2], fine.len(), cap).unwrap();
    let balances = BalanceProbePlan::new(probes, fine.len() + 2, cap).unwrap();
    let plan = QuadraturePlan::new(balances, sets, 1000, cap).unwrap();
    assert!(QuadraturePlan::new(balances, sets, 1, cap).is_err());
    let stats = admission.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    let construction = Region::new(GLOBAL);
    let mut owner = ProbeFamily::new(probes).unwrap();
    let mut consumer = BalanceQuadrature::new(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let work = Region::new(GLOBAL);
    assert!(consumer.measure(&owner).is_err());
    for time in fine {
        owner.advance().unwrap().unwrap();
        assert_eq!(consumer.measure(&owner).unwrap().clock(), time);
    }
    assert!(consumer.report().is_some());
    assert!(consumer.measure(&owner).is_err());
    let stats = work.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
}
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).unwrap()
}
