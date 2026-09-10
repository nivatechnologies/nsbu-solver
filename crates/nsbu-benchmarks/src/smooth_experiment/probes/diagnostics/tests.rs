//! A second-child failure must discard the aggregate while preserving every earlier legal commit.
use super::*;
use crate::smooth_experiment::{probes::ProbePlan, FamilyPlan, FamilySettings};
use nsbu_solver::{
    domain::Layout, integrators::indicator::Tolerances, verification::times::TestedTimes,
};

#[test]
fn pressure_failure_after_physical_sampling_publishes_nothing_and_terminates() {
    let accepted = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let times = [0, 7, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let cap = 128 * 1024 * 1024;
    let base = FamilyPlan::new(
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
        TestedTimes::new(&accepted, 3).unwrap(),
        cap,
    )
    .unwrap();
    let probes = ProbePlan::new(base, TestedTimes::new(&times, 3).unwrap(), 3, cap).unwrap();
    let plan =
        ProbeDiagnosticsPlan::new(probes, Layout::new([24; 3]).unwrap(), [1.0; 6], 3, cap).unwrap();
    let mut family = ProbeFamily::new(probes).unwrap();
    family.advance().unwrap();
    let clocks = std::array::from_fn::<_, 6, _>(|i| family.branch(i).unwrap().state().clock());
    let mut diagnostics = ProbeDiagnostics::new(plan).unwrap();
    // Exhaust only the privately owned pressure consumer; its ledger cannot be reset.
    for _ in 0..3 {
        diagnostics.pressure.probe_quantities(&family).unwrap();
    }
    assert!(diagnostics.measure(&family).is_err());
    assert_eq!(diagnostics.charged_work().physical.attempts, 1);
    assert_eq!(diagnostics.charged_work().pressure.attempts, 3);
    assert!(diagnostics.is_terminated());
    assert_eq!(diagnostics.next_time(), Some(times[0]));
    assert_eq!(
        clocks,
        std::array::from_fn(|i| family.branch(i).unwrap().state().clock())
    );
    let spent = diagnostics.charged_work();
    assert!(matches!(
        diagnostics.measure(&family),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(diagnostics.charged_work(), spent);
}
