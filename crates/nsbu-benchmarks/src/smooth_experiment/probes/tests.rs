//! A later diagnostic failure must invalidate previous current views without undoing legal commits.
use super::*;
use crate::smooth_experiment::{FamilyPlan, FamilySettings};
use nsbu_solver::{integrators::indicator::Tolerances, verification::times::TestedTimes};
#[test]
fn a_desynchronized_accepted_history_cannot_reuse_the_last_complete_probe_views() {
    let accepted = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let clocks = [0, 7, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
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
    let cap = 128 * 1024 * 1024;
    let family = FamilyPlan::new(settings, TestedTimes::new(&accepted, 3).unwrap(), cap).unwrap();
    let plan = ProbePlan::new(family, TestedTimes::new(&clocks, 3).unwrap(), 3, cap).unwrap();
    let mut run = ProbeFamily::new(plan).unwrap();
    let initial = run.advance().unwrap().unwrap();
    assert_eq!(initial.clock(), clocks[0]);
    assert!(run.fields(2).is_some());
    // Inject one additional legal private-owner commit, making its ring too new for probe seven.
    assert!(matches!(
        run.branches[2].step().unwrap(),
        Outcome::Committed(_)
    ));
    let committed = run.branches[2].state().clock();
    assert_eq!(committed.elapsed(), 48);
    assert!(run.advance().is_err());
    assert!(run.is_terminated());
    assert!(run.fields(0).is_none());
    assert!(run.fields(2).is_none());
    assert_eq!(run.branch(2).unwrap().state().clock(), committed);
    assert_eq!(run.next_time(), Some(clocks[1]));
    assert_eq!(run.charged_work().attempts, 2);
    assert!(matches!(run.advance(), Err(FamilyError::Terminated)));
}
