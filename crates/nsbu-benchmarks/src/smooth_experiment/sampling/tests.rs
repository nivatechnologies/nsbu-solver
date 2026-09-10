//! Failure after one child has advanced cannot produce a partial aggregate or a fresh retry.
use super::*;
use crate::smooth_experiment::{FamilyPlan, FamilySettings};
use nsbu_solver::{
    domain::Layout, integrators::indicator::Tolerances, verification::times::TestedTimes,
};

#[test]
fn a_desynchronized_child_terminates_the_complete_sampling_consumer() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
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
    let family_plan =
        FamilyPlan::new(settings, TestedTimes::new(&clocks, 3).unwrap(), cap).unwrap();
    let samples = [24, 32, 36].map(|n| Layout::new([n; 3]).unwrap());
    let plan = SamplingPlan::new(family_plan, samples, [1e-8; 6], 4, cap).unwrap();
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let mut consumer = SamplingWorkspace::new(plan).unwrap();
    family.advance().unwrap();
    // Deliberately advance only a private child to simulate an inconsistent diagnostic owner.
    consumer.physical[1].measure(&family).unwrap();
    assert!(consumer.measure(&family).is_err());
    assert!(consumer.is_terminated());
    assert_eq!(consumer.next_time(), Some(clocks[0]));
    assert_eq!(consumer.remaining(), 3);
    let charged = consumer.charged_work();
    assert_eq!(consumer.physical[0].next_time(), Some(clocks[1]));
    assert_eq!(consumer.pressure[0].next_time(), Some(clocks[1]));
    assert!(matches!(
        consumer.measure(&family),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(consumer.charged_work(), charged);
    assert_eq!(family.branch(2).unwrap().state().clock(), clocks[0]);
}
