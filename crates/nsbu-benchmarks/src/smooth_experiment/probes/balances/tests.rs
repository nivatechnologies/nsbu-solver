//! Each exhausted child must preserve charged work and refuse a partial balance publication.
use super::*;
use crate::smooth_experiment::{probes::ProbePlan, FamilyPlan, FamilySettings};
use nsbu_solver::{integrators::indicator::Tolerances, verification::times::TestedTimes};
const CAP: usize = 128 * 1024 * 1024;
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).unwrap()
}
fn probes<'a>(accepted: &'a [TickClock], times: &'a [TickClock]) -> ProbePlan<'a> {
    let settings = FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        viscosity: 1.0,
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    };
    let family = FamilyPlan::new(
        settings,
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(times, times.len()).unwrap(),
        times.len(),
        CAP,
    )
    .unwrap()
}
#[test]
fn later_child_exhaustion_retains_work_and_previous_sample_without_partial_publication() {
    for failing_branch in 0..6 {
        exhausted_child(failing_branch);
    }
}
fn exhausted_child(failing_branch: usize) {
    let accepted = [0, 64, 128].map(clock);
    let times = [0, 7, 128].map(clock);
    let owner_plan = probes(&accepted, &times);
    let plan = BalanceProbePlan::new(owner_plan, 3, CAP).unwrap();
    let mut owner = ProbeFamily::new(owner_plan).unwrap();
    let mut consumer = BalanceProbes::new(plan).unwrap();
    assert_eq!(consumer.next_time(), Some(times[0]));
    assert_eq!(consumer.remaining(), 3);
    owner.advance().unwrap();
    let original = consumer.measure(&owner).unwrap();
    let fields = owner.fields(failing_branch).unwrap();
    for _ in 0..2 {
        consumer.children[failing_branch]
            .sample_probe(fields.domain, fields.clock, fields.value)
            .unwrap();
    }
    owner.advance().unwrap();
    let clocks = std::array::from_fn::<_, 6, _>(|i| owner.branch(i).unwrap().state().clock());
    assert!(consumer.measure(&owner).is_err());
    assert!(consumer.is_terminated());
    assert_eq!(consumer.charged_work().attempts, 2);
    assert_eq!(consumer.remaining(), 1);
    let expected = std::array::from_fn(|i| match i.cmp(&failing_branch) {
        std::cmp::Ordering::Less => 2,
        std::cmp::Ordering::Equal => 3,
        std::cmp::Ordering::Greater => 1,
    });
    assert_eq!(consumer.child_work().map(|w| w.samples), expected);
    assert_eq!(original.clock(), times[0]);
    assert_eq!(*original.branches(), [BalanceSample::REST; 6]);
    let charged = consumer.charged_work();
    assert!(matches!(
        consumer.measure(&owner),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(charged, consumer.charged_work());
    assert_eq!(
        clocks,
        std::array::from_fn(|i| owner.branch(i).unwrap().state().clock())
    );
}
#[test]
fn quadrature_failure_rolls_back_all_earlier_pending_level_updates_and_terminates() {
    use super::quadrature::{BalanceQuadrature, QuadraturePlan};
    let accepted = [0, 64, 128].map(clock);
    let middle = [0, 32, 64, 96, 128].map(clock);
    let fine = [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let owner_plan = probes(&accepted, &fine);
    let balance = BalanceProbePlan::new(owner_plan, 9, CAP).unwrap();
    let sets =
        [&accepted[..], &middle[..], &fine[..]].map(|s| TestedTimes::new(s, s.len()).unwrap());
    let plan = QuadraturePlan::new(balance, sets, 1000, CAP).unwrap();
    let mut owner = ProbeFamily::new(owner_plan).unwrap();
    let mut quad = BalanceQuadrature::new(plan).unwrap();
    for _ in 0..4 {
        owner.advance().unwrap();
        quad.measure(&owner).unwrap();
    }
    // A corrupted pending midpoint makes the next coarse Simpson span invalid after three
    // earlier branches have staged valid updates. Only this private negative test injects it.
    quad.inject_invalid_midpoint_for_test(clock(7));
    let counts = quad.sample_counts();
    owner.advance().unwrap();
    assert!(quad.measure(&owner).is_err());
    assert!(quad.is_terminated());
    assert!(quad.report().is_none());
    assert_eq!(quad.sample_counts(), counts);
    let charged = quad.charged_work();
    assert_eq!(charged.attempts, 5);
    assert!(matches!(quad.measure(&owner), Err(FamilyError::Terminated)));
    assert_eq!(quad.charged_work(), charged);
}
