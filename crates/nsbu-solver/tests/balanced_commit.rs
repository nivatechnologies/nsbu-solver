//! Diagnostic refusal is transactional; a copied physical/balance history resumes coherently.
mod mean_balance;
mod source_contract;
mod trajectory_plan;
use nsbu_solver::{
    diagnostics::history::BalanceHistory,
    domain::{Epoch, SpectralState, TickClock},
    experiment::commit_balanced,
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        transaction::{AcceptedAttempt, CandidateState},
    },
    lineage::PhysicalImage,
    Complex64, SolverError,
};

fn attempt(
    state: &SpectralState,
    candidate: &mut CandidateState,
    work: &mut AttemptWorkspace,
) -> AcceptedAttempt {
    let mut source = source_contract::Source::new(
        usize::MAX,
        [[Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)]; 3],
    );
    work.try_advance(
        state,
        candidate,
        4,
        Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        },
        &mut source,
    )
    .unwrap()
    .accepted
    .unwrap()
}

#[test]
fn failed_diagnostics_publish_neither_physical_nor_balance_history() {
    let plan = trajectory_plan::plan();
    let zero = TickClock::from_rest(-12, 100).unwrap();
    let mut state = SpectralState::from_rest(plan, zero, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, zero, Epoch(0)).unwrap();
    let mut work = AttemptWorkspace::new(plan).unwrap();
    let mut history = BalanceHistory::new(zero, mean_balance::measured(&state), 3).unwrap();
    let initial = history;
    let token = attempt(&state, &mut candidate, &mut work);
    let mut sample = mean_balance::measured(candidate.proposal(&state, &token).unwrap());
    sample.forcing_work = f64::NAN;
    assert_eq!(
        commit_balanced(&mut state, &mut candidate, token, &mut history, sample),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(state.clock(), zero);
    assert_eq!(state.accepted_steps(), 0);
    assert_eq!(history.clock(), zero);
    assert_eq!(history.samples(), 1);
    let token = attempt(&state, &mut candidate, &mut work);
    let sample = mean_balance::measured(candidate.proposal(&state, &token).unwrap());
    commit_balanced(&mut state, &mut candidate, token, &mut history, sample).unwrap();
    assert_eq!(state.clock(), history.clock());
    assert_eq!(state.accepted_steps(), 1);
    assert!(history.has_pending_midpoint());
    for mut mismatched in [
        initial,
        initial
            .with_sample(zero.stages(4).unwrap()[2], mean_balance::measured(&state))
            .unwrap()
            .with_sample(state.clock(), mean_balance::measured(&state))
            .unwrap(),
    ] {
        let token = attempt(&state, &mut candidate, &mut work);
        let sample = mean_balance::measured(candidate.proposal(&state, &token).unwrap());
        assert_eq!(
            commit_balanced(&mut state, &mut candidate, token, &mut mismatched, sample),
            Err(SolverError::InvalidPayload)
        );
        assert_eq!(state.accepted_steps(), 1);
        assert_eq!(history.samples(), 2);
    }
    let image = PhysicalImage::capture(&state, PhysicalImage::reservation(plan).unwrap()).unwrap();
    let mut restored = image.into_state();
    let mut restored_history = history;
    let mut restored_candidate = CandidateState::new(plan, zero, Epoch(0)).unwrap();
    let mut restored_work = AttemptWorkspace::new(plan).unwrap();
    for (physical, proposed, scratch, balance) in [
        (&mut state, &mut candidate, &mut work, &mut history),
        (
            &mut restored,
            &mut restored_candidate,
            &mut restored_work,
            &mut restored_history,
        ),
    ] {
        let token = attempt(physical, proposed, scratch);
        let sample = mean_balance::measured(proposed.proposal(physical, &token).unwrap());
        commit_balanced(physical, proposed, token, balance, sample).unwrap();
        assert_eq!(physical.clock(), balance.clock());
        assert_eq!(physical.accepted_steps(), 2);
        assert_eq!(balance.samples(), 3);
    }
    let a = history.integral().unwrap();
    let b = restored_history.integral().unwrap();
    assert_eq!(a.energy_rhs.to_bits(), b.energy_rhs.to_bits());
    assert_eq!(a.energy_defect.to_bits(), b.energy_defect.to_bits());
    assert_eq!(state.component(0).unwrap(), restored.component(0).unwrap());
}
