//! Partial child progress and stale accepted-node origins cannot escape as complete residual evidence.
use super::*;
use crate::smooth_experiment::{probes::ProbePlan, FamilyPlan, FamilySettings};
use nsbu_solver::{
    experiment::control::Outcome, integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
const CAP: usize = 128 * 1024 * 1024;
fn clock(t: u128) -> TickClock {
    TickClock::restore(-16, 512, t, 512 - t).unwrap()
}
fn plan<'a>(
    accepted: &'a [TickClock],
    times: &'a [TickClock],
    subset: &'a [TickClock],
) -> ResidualFamilyPlan<'a> {
    let family = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            viscosity: 1.0,
            endpoint: 128,
            tolerances: Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(accepted, 3).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(family, TestedTimes::new(times, 3).unwrap(), 3, CAP).unwrap();
    ResidualFamilyPlan::new(probes, subset, 1, CAP).unwrap()
}
#[test]
fn a_later_child_failure_retains_charges_and_never_publishes_partial_comparisons() {
    let accepted = [0, 64, 128].map(clock);
    let times = [0, 7, 128].map(clock);
    let plan = plan(&accepted, &times, &times[1..2]);
    let mut family = ProbeFamily::new(plan.probe_plan()).unwrap();
    family.advance().unwrap();
    family.advance().unwrap();
    let mut residuals = ResidualFamily::new(plan).unwrap();
    residuals.children[1]
        .measure(family.branch(1).unwrap(), times[1])
        .unwrap();
    assert!(residuals.measure(&family).is_err());
    assert!(residuals.is_terminated());
    assert_eq!(residuals.child_work().map(|w| w.probes), [1, 1, 0, 0, 0, 0]);
    assert_eq!(residuals.charged_work(), plan.bounds().work);
    assert_eq!(residuals.next_time(), Some(times[1]));
    let spent = residuals.charged_work();
    assert!(matches!(
        residuals.measure(&family),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(residuals.charged_work(), spent);
}
#[test]
fn a_new_legal_history_cannot_be_relabelled_as_the_previously_published_probe_origin() {
    let accepted = [0, 64, 128].map(clock);
    let times = [0, 31, 128].map(clock);
    let plan = plan(&accepted, &times, &times[1..2]);
    let mut family = ProbeFamily::new(plan.probe_plan()).unwrap();
    family.advance().unwrap();
    family.advance().unwrap();
    assert!(matches!(
        family.branches[2].step().unwrap(),
        Outcome::Committed(_)
    ));
    assert_eq!(family.branches[2].state().clock().elapsed(), 48);
    let mut residuals = ResidualFamily::new(plan).unwrap();
    assert!(residuals.measure(&family).is_err());
    assert!(residuals.is_terminated());
    assert_eq!(residuals.child_work().map(|w| w.probes), [1, 1, 1, 0, 0, 0]);
    assert_eq!(family.branches[2].state().clock().elapsed(), 48);
}
