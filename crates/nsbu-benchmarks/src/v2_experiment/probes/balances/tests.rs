//! Private failure control for transactional exact-v2 quadrature histories.
use super::{
    quadrature::{V2BalanceQuadrature, V2QuadraturePlan},
    V2BalancePlan,
};
use crate::{
    runtime_force::ForceSettings,
    v2_experiment::{probes::ProbePlan, FamilyError, FamilyPlan, FamilySettings},
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
const CAP: usize = 256 * 1024 * 1024;
fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn plan<'a>(accepted: &'a [TickClock], fine: &'a [TickClock]) -> ProbePlan<'a> {
    let family = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([12; 3]).unwrap(),
                workers: 0,
            },
            endpoint: 128,
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(fine, fine.len()).unwrap(),
        fine.len(),
        CAP,
    )
    .unwrap()
}

#[test]
fn failed_late_history_update_rolls_back_all_levels_and_terminates() {
    let accepted = [0, 64, 128].map(clock);
    let coarse = [0, 64, 128].map(clock);
    let middle = [0, 32, 64, 96, 128].map(clock);
    let fine = [0, 16, 32, 48, 64, 80, 96, 112, 128].map(clock);
    let probes = plan(&accepted, &fine);
    let balance = V2BalancePlan::new(probes, fine.len(), CAP).unwrap();
    let sets = [&coarse[..], &middle[..], &fine[..]]
        .map(|values| TestedTimes::new(values, values.len()).unwrap());
    let admitted = V2QuadraturePlan::new(balance, sets, 10_000, CAP).unwrap();
    let mut owner = super::super::ProbeFamily::new(probes).unwrap();
    let mut quadrature = V2BalanceQuadrature::new(admitted).unwrap();
    for _ in 0..4 {
        owner.advance().unwrap();
        quadrature.measure(&owner).unwrap();
    }
    quadrature.inject_invalid_midpoint_for_test(clock(7));
    let counts = quadrature.sample_counts();
    owner.advance().unwrap();
    assert!(quadrature.measure(&owner).is_err());
    assert_eq!(quadrature.sample_counts(), counts);
    assert!(quadrature.report().is_none());
    assert!(quadrature.is_terminated());
    let charged = quadrature.charged_work();
    assert!(matches!(
        quadrature.measure(&owner),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(quadrature.charged_work(), charged);
}
