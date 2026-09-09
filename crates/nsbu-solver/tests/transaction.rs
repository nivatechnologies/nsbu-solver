//! Exact stage requests, fine-result commits, rollback and stale acceptance tokens.
mod support;
mod transaction_support;
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace,
    indicator::Tolerances,
    kernel::{RhsBounds, RightHandSide},
    transaction::{commit_candidate, CandidateState},
};
use nsbu_solver::{Complex64, SolverError};
use support::CallLog;

struct MeanSource {
    log: CallLog,
    oscillatory: bool,
}
impl RightHandSide for MeanSource {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: std::mem::size_of::<Self>() + 16 * self.log.ticks.capacity(),
            work_units: 145,
            scalar_transforms: 0,
        })
    }

    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.log.record(time)?;
        let value = if self.oscillatory {
            (time.elapsed() as f64).cos()
        } else {
            1.0
        };
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
            values[0] = Complex64::new(value, 0.0);
        }
        Ok(())
    }
}
fn plan(epoch: Epoch) -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 2048,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 4096,
        },
        1024 * 1024,
        epoch,
    )
    .unwrap()
}
fn policy() -> Tolerances {
    Tolerances {
        absolute: [1e-10; 2],
        relative: [1e-7; 2],
    }
}

#[test]
fn twelve_exact_requests_commit_fine_state_by_swapping_owned_buffers() {
    let plan = plan(Epoch(0));
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut work = AttemptWorkspace::new(plan).unwrap();
    let old = state.component(0).unwrap().as_ptr();
    let mut rhs = MeanSource {
        log: CallLog {
            ticks: Vec::with_capacity(24),
            fail_at: 0,
        },
        oscillatory: false,
    };
    let result = work
        .try_advance(&state, &mut candidate, 8, policy(), &mut rhs)
        .unwrap();
    assert_eq!(rhs.log.ticks, [0, 4, 4, 8, 0, 2, 2, 4, 4, 6, 6, 8]);
    assert_eq!((result.rhs_calls, result.ticks), (12, 8));
    assert!(result.indicators.errors[0] < 1e-15);
    assert_eq!(result.indicators.errors[1], 0.0);
    assert_eq!(state.clock(), clock);
    commit_candidate(plan, &mut state, &mut candidate, result.accepted.unwrap()).unwrap();
    assert_ne!(state.component(0).unwrap().as_ptr(), old);
    assert_eq!(
        (
            state.clock().elapsed(),
            state.accepted_steps(),
            state.epoch()
        ),
        (8, 1, Epoch(1))
    );
    for axis in 0..3 {
        assert!((state.component(axis).unwrap()[0].re - 0.125).abs() < 1e-15);
    }
    let second = work
        .try_advance(&state, &mut candidate, 8, policy(), &mut rhs)
        .unwrap();
    commit_candidate(plan, &mut state, &mut candidate, second.accepted.unwrap()).unwrap();
    assert_eq!(state.component(0).unwrap().as_ptr(), old);
    assert_eq!(state.clock().elapsed(), 16);
}

#[test]
fn rejected_and_failed_attempts_preserve_every_committed_value() {
    let plan = plan(Epoch(0));
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let (state, mut candidate, mut work) = transaction_support::setup(plan, clock);
    let mut rhs = MeanSource {
        log: CallLog {
            ticks: Vec::with_capacity(24),
            fail_at: 0,
        },
        oscillatory: true,
    };
    let result = work
        .try_advance(&state, &mut candidate, 8, policy(), &mut rhs)
        .unwrap();
    assert!(result.accepted.is_none());
    assert!(result.indicators.ratios[0] > 1.0);
    rhs.log.fail_at = 16;
    assert!(matches!(
        work.try_advance(&state, &mut candidate, 8, policy(), &mut rhs),
        Err(SolverError::ResourceLimit)
    ));
    assert_eq!(
        (state.clock(), state.epoch(), state.accepted_steps()),
        (clock, Epoch(0), 0)
    );
    for axis in 0..3 {
        assert!(state
            .component(axis)
            .unwrap()
            .iter()
            .all(|v| *v == Complex64::new(0.0, 0.0)));
    }
}

#[test]
fn tokens_refuse_reused_candidates_other_states_and_changed_plans() {
    let initial_plan = plan(Epoch(0));
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let mut state = SpectralState::from_rest(initial_plan, clock, Epoch(0)).unwrap();
    let mut other = SpectralState::from_rest(initial_plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(initial_plan, clock, Epoch(0)).unwrap();
    let mut work = AttemptWorkspace::new(initial_plan).unwrap();
    let mut rhs = MeanSource {
        log: CallLog {
            ticks: Vec::with_capacity(60),
            fail_at: 0,
        },
        oscillatory: false,
    };
    for case in 0..3 {
        let token = work
            .try_advance(&state, &mut candidate, 8, policy(), &mut rhs)
            .unwrap()
            .accepted
            .unwrap();
        let result = match case {
            0 => {
                let _replacement = work
                    .try_advance(&state, &mut candidate, 8, policy(), &mut rhs)
                    .unwrap();
                commit_candidate(initial_plan, &mut state, &mut candidate, token)
            }
            1 => commit_candidate(initial_plan, &mut other, &mut candidate, token),
            _ => commit_candidate(plan(Epoch(1)), &mut state, &mut candidate, token),
        };
        assert_eq!(result, Err(SolverError::StaleAttempt));
        assert_eq!(state.clock(), clock);
        assert_eq!(other.clock(), clock);
    }
}

#[test]
fn every_full_and_half_step_failure_rolls_back_the_committed_trajectory() {
    let plan = plan(Epoch(0));
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let (state, mut candidate, mut work) = transaction_support::setup(plan, clock);
    for fail_at in 1..=12 {
        let mut rhs = MeanSource {
            log: CallLog {
                ticks: Vec::with_capacity(12),
                fail_at,
            },
            oscillatory: false,
        };
        assert!(matches!(
            work.try_advance(&state, &mut candidate, 8, policy(), &mut rhs),
            Err(SolverError::ResourceLimit)
        ));
        assert_eq!(rhs.log.ticks.len(), fail_at);
        assert_eq!(state.clock(), clock);
        assert_eq!(state.accepted_steps(), 0);
    }
}

#[test]
fn finite_scheduler_retries_only_local_error_and_never_silently_changes_a_core_request() {
    use nsbu_solver::integrators::scheduler::retry_local;
    let plan = plan(Epoch(0));
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let (state, mut candidate, mut work) = transaction_support::setup(plan, clock);
    for (ticks, cap, oscillatory, expected) in [
        (8, 0, false, SolverError::InvalidStep),
        (8, 65, false, SolverError::InvalidStep),
        (8, 1, true, SolverError::RetryLimit),
        (4, 2, true, SolverError::ClockCapacityExceeded),
        (2, 2, false, SolverError::InvalidStep),
    ] {
        let mut rhs = MeanSource {
            log: CallLog {
                ticks: Vec::with_capacity(24),
                fail_at: 0,
            },
            oscillatory,
        };
        assert_eq!(
            (retry_local(
                &mut work,
                &state,
                &mut candidate,
                ticks,
                policy(),
                cap,
                &mut rhs
            ))
            .unwrap_err(),
            expected
        );
        assert_eq!(state.clock(), clock);
    }
    let mut rhs = MeanSource {
        log: CallLog {
            ticks: Vec::with_capacity(24),
            fail_at: 0,
        },
        oscillatory: false,
    };
    let result = retry_local(&mut work, &state, &mut candidate, 8, policy(), 64, &mut rhs).unwrap();
    assert_eq!(result.ticks, 8);
    assert!(result.accepted.is_some());
}
