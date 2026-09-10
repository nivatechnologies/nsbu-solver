//! Simulated stale/corrupt internal holders exercise every pre-swap identity check.
use super::*;
use crate::{
    domain::{Domain, ExtraStorage},
    Complex64,
};

use crate::test_support::transaction_setup as setup;

#[test]
fn every_private_acceptance_identity_is_validated_before_any_payload_exchange() {
    for case in 0..9 {
        let (plan, mut state, mut candidate) = setup();
        let accepted = candidate.accept(&state);
        match case {
            0 => state.components[0][0] = Complex64::new(1.0, 2.0),
            1 => candidate.state.components[1][0] = Complex64::new(2.0, 3.0),
            2 => candidate.accepted = None,
            3 => candidate.generation = Epoch(1),
            4 => {
                candidate.state.plan = ResourcePlan::new(
                    plan.domain(),
                    ExtraStorage {
                        fft: 0,
                        force: 0,
                        diagnostics: 0,
                        overhead: 4096,
                    },
                    1024 * 1024,
                    Epoch(1),
                )
                .unwrap()
            }
            5 => state.epoch = Epoch(1),
            6 => state.clock = state.clock.stages(4).unwrap()[4],
            7 => candidate.state.components[0].reserve_exact(1),
            _ => state.accepted_steps = 1,
        }
        let before = stamp(&state);
        let candidate_before = stamp(&candidate.state);
        assert_eq!(
            candidate.proposal(&state, &accepted).unwrap_err(),
            SolverError::StaleAttempt
        );
        assert_eq!(
            commit_candidate(plan, &mut state, &mut candidate, accepted),
            Err(SolverError::StaleAttempt)
        );
        assert_eq!(stamp(&state), before);
        assert_eq!(stamp(&candidate.state), candidate_before);
    }
}

#[test]
fn exhausted_candidate_generation_invalidates_prior_acceptance() {
    let (_, state, mut candidate) = setup();
    let _token = candidate.accept(&state);
    candidate.generation = Epoch(u128::MAX);
    assert_eq!(candidate.invalidate(), Err(SolverError::EpochExhausted));
    assert_eq!(candidate.accepted, None);
}

#[test]
fn coefficient_fingerprint_matches_independently_computed_bitstream_fixture() {
    let (_, mut state, _) = setup();
    state.components[0][0] = Complex64::new(1.0, 2.0);
    state.components[1][0] = Complex64::new(-3.0, 4.0);
    assert_eq!(stamp(&state).digest, 14360686265443977637_u64);
}

struct Zero;
impl crate::integrators::kernel::RightHandSide for Zero {
    fn bounds(&self) -> Option<crate::integrators::kernel::RhsBounds> {
        Some(crate::integrators::kernel::RhsBounds {
            storage_bytes: 0,
            work_units: 144,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for value in output.into_iter().flatten() {
            *value = Complex64::new(0.0, 0.0);
        }
        Ok(())
    }
}

#[test]
fn accepted_step_counter_exhaustion_does_not_change_committed_time_or_storage() {
    use crate::integrators::{attempt::AttemptWorkspace, indicator::Tolerances};
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 4096,
        },
        1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let (_, mut state, _) = setup();
    state.plan = plan;
    let clock = state.clock;
    state.accepted_steps = u128::MAX;
    let before = stamp(&state);
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let inadequate = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 0,
            overhead: 4096,
        },
        1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    assert!(matches!(
        AttemptWorkspace::new(inadequate),
        Err(SolverError::ResourceLimit)
    ));
    let mut work = AttemptWorkspace::new(plan).unwrap();
    use crate::integrators::kernel::RightHandSide;
    let mut undeclared = Unknown;
    assert_eq!(
        undeclared.evaluate([&[]; 3], clock, [&mut [], &mut [], &mut []]),
        Err(SolverError::UnknownProviderCost)
    );
    let result = work.try_advance(
        &state,
        &mut candidate,
        4,
        Tolerances {
            absolute: [1e-10; 2],
            relative: [1e-7; 2],
        },
        &mut undeclared,
    );
    assert_eq!(result.unwrap_err(), SolverError::UnknownProviderCost);

    for generation in [Epoch(0), Epoch(u128::MAX)] {
        candidate.generation = generation;
        let result = work.try_advance(
            &state,
            &mut candidate,
            4,
            Tolerances {
                absolute: [1e-10; 2],
                relative: [1e-7; 2],
            },
            &mut Zero,
        );
        assert_eq!(result.unwrap_err(), SolverError::EpochExhausted);
        assert_eq!(stamp(&state), before);
        assert_eq!(candidate.accepted, None);
    }
}

#[test]
fn matching_private_metadata_commits_and_a_wrong_plan_does_not() {
    let (plan, mut state, mut candidate) = setup();
    candidate.state.clock = state.clock.stages(4).unwrap()[4];
    candidate.state.epoch = Epoch(1);
    candidate.state.accepted_steps = 1;
    let accepted = candidate.accept(&state);
    commit_candidate(plan, &mut state, &mut candidate, accepted).unwrap();
    assert_eq!(state.clock.elapsed(), 4);
    let other = ResourcePlan::new(
        plan.domain(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 0,
            overhead: 4096,
        },
        1024 * 1024,
        Epoch(2),
    )
    .unwrap();
    candidate.state.plan = other;
    let accepted = candidate.accept(&state);
    assert_eq!(
        commit_candidate(other, &mut state, &mut candidate, accepted),
        Err(SolverError::StaleAttempt)
    );
}

struct Unknown;
impl crate::integrators::kernel::RightHandSide for Unknown {
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _clock: TickClock,
        _output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        Err(SolverError::UnknownProviderCost)
    }
}
