//! Bounded trajectory progress, rejection and partial-failure evidence.
mod mean_source;
mod source_contract;
use mean_source::source;
mod trajectory_plan;
mod transaction_support;
use nsbu_solver::{
    domain::TickClock,
    integrators::{
        indicator::Tolerances,
        trajectory::{FixedRun, RunLimits, StopReason},
    },
    SolverError,
};

fn limits() -> RunLimits {
    RunLimits {
        endpoint: 12,
        step_ticks: 4,
        maximum_attempts: 3,
    }
}

#[test]
fn finite_work_and_quarter_tick_admission() {
    let clock = TickClock::from_rest(-12, 64).unwrap();
    assert_eq!(limits().attempts(clock).unwrap(), 3);
    let resumed = TickClock::restore(-12, 64, 4, 60).unwrap();
    assert_eq!(limits().attempts(resumed).unwrap(), 2);
    assert_eq!(
        RunLimits {
            maximum_attempts: usize::MAX / 12,
            ..limits()
        }
        .attempts(clock)
        .unwrap(),
        3
    );
    for (settings, error) in [
        (
            RunLimits {
                endpoint: 0,
                ..limits()
            },
            SolverError::InvalidClock,
        ),
        (
            RunLimits {
                endpoint: 64,
                ..limits()
            },
            SolverError::InvalidClock,
        ),
        (
            RunLimits {
                step_ticks: 0,
                ..limits()
            },
            SolverError::InvalidStep,
        ),
        (
            RunLimits {
                step_ticks: 2,
                ..limits()
            },
            SolverError::InvalidStep,
        ),
        (
            RunLimits {
                endpoint: 13,
                ..limits()
            },
            SolverError::InvalidStep,
        ),
        (
            RunLimits {
                maximum_attempts: 2,
                ..limits()
            },
            SolverError::RetryLimit,
        ),
        (
            RunLimits {
                maximum_attempts: usize::MAX,
                ..limits()
            },
            SolverError::RetryLimit,
        ),
    ] {
        assert_eq!(settings.attempts(clock).unwrap_err(), error);
    }
    let huge = TickClock::from_rest(-100, 1 << 100).unwrap();
    assert_eq!(
        RunLimits {
            endpoint: 1 << 99,
            step_ticks: 4,
            maximum_attempts: 3
        }
        .attempts(huge)
        .unwrap_err(),
        SolverError::SizeOverflow
    );
}

#[test]
fn complete_run_and_mid_run_refusal_preserve_actual_clock() {
    for (fail_at, reason, committed, attempted) in [
        (usize::MAX, StopReason::EndpointReached, 3, 3),
        (
            13,
            StopReason::Refused(SolverError::ProviderBudgetExceeded),
            1,
            2,
        ),
    ] {
        let (mut state, mut candidate, mut workspace) = transaction_support::setup(
            trajectory_plan::plan(),
            TickClock::from_rest(-12, 64).unwrap(),
        );
        let mut source = source(fail_at);
        let mut runner = FixedRun::new(&mut state, &mut candidate, &mut workspace, &mut source);
        let report = runner
            .execute(
                limits(),
                Tolerances {
                    absolute: [1.0; 2],
                    relative: [0.0; 2],
                },
            )
            .unwrap();
        assert_eq!(report.reason, reason);
        assert_eq!(report.committed, committed);
        assert_eq!(report.attempted, attempted);
        assert_eq!(report.clock.elapsed(), 4 * committed as u128);
        assert_eq!(state.clock(), report.clock);
        assert_eq!(state.accepted_steps(), committed as u128);
        assert!(report.maximum_local_ratios[0] > 0.0);
        assert_eq!(report.maximum_local_ratios[1], 0.0);
    }
}

#[test]
fn configuration_and_numerical_rejection_do_not_commit() {
    let (mut state, mut candidate, mut workspace) = transaction_support::setup(
        trajectory_plan::plan(),
        TickClock::from_rest(-12, 64).unwrap(),
    );
    let mut source = source(usize::MAX);
    let mut runner = FixedRun::new(&mut state, &mut candidate, &mut workspace, &mut source);
    assert_eq!(
        runner
            .execute(
                limits(),
                Tolerances {
                    absolute: [0.0; 2],
                    relative: [0.0; 2]
                }
            )
            .unwrap_err(),
        SolverError::InvalidStep
    );
    assert_eq!(
        runner
            .execute(
                RunLimits {
                    endpoint: 0,
                    ..limits()
                },
                Tolerances {
                    absolute: [1.0; 2],
                    relative: [0.0; 2]
                }
            )
            .unwrap_err(),
        SolverError::InvalidClock
    );
    let report = runner
        .execute(
            limits(),
            Tolerances {
                absolute: [1e-30; 2],
                relative: [0.0; 2],
            },
        )
        .unwrap();
    assert_eq!(report.reason, StopReason::LocalErrorRejected);
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 0);
    assert_eq!(report.clock.elapsed(), 0);
    assert!(report.maximum_local_ratios[0] > 1.0);
    assert_eq!(state.accepted_steps(), 0);
}
