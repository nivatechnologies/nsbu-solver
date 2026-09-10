//! A controller snapshot retains exact scheduling, counts, budgets and terminal outcomes.
use nsbu_solver::{
    domain::TickClock,
    experiment::control::{Configuration, Controller, Outcome},
    integrators::{
        indicator::{Indicators, Tolerances},
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
    SolverError,
};

fn configuration() -> Configuration {
    Configuration {
        limits: RunLimits {
            endpoint: 12,
            step_ticks: 4,
            maximum_attempts: 4,
        },
        method: Method::HochbruckOstermann,
        tolerances: Tolerances {
            absolute: [0.5, 0.25],
            relative: [0.125, 0.0625],
        },
    }
}
fn clock() -> TickClock {
    TickClock::from_rest(-12, 100).unwrap()
}
fn indicators(ratios: [f64; 2]) -> Indicators {
    Indicators {
        errors: [0.25, 0.125],
        ratios,
    }
}

#[test]
fn snapshots_preserve_frozen_configuration_and_stop_at_the_exact_endpoint() {
    let mut controller = Controller::new(clock(), configuration()).unwrap();
    let config = controller.configuration();
    assert_eq!(controller.required_commits(), 3);
    assert_eq!(config.method, Method::HochbruckOstermann);
    assert_eq!(config.limits.endpoint, 12);
    assert_eq!(config.limits.step_ticks, 4);
    assert_eq!(config.limits.maximum_attempts, 4);
    assert_eq!(config.tolerances.absolute, [0.5, 0.25]);
    assert_eq!(config.tolerances.relative, [0.125, 0.0625]);
    assert_eq!(controller.attempted(), 0);
    assert_eq!(controller.committed(), 0);
    for count in 1..=3 {
        assert_eq!(controller.stopped(), None);
        assert_eq!(
            controller.next_clock().unwrap().elapsed(),
            4 * count as u128
        );
        let snapshot = controller;
        let outcome = Outcome::Committed(indicators([0.0, 1.0]));
        controller = controller.with_outcome(outcome).unwrap();
        let restored = snapshot.with_outcome(outcome).unwrap();
        assert_eq!(restored.clock(), controller.clock());
        assert_eq!(restored.attempted(), count);
        assert_eq!(restored.committed(), count);
        assert_eq!(restored.attempts_left(), 4 - count);
        assert_eq!(restored.stopped(), controller.stopped());
    }
    assert_eq!(controller.stopped(), Some(StopReason::EndpointReached));
    assert_eq!(controller.next_clock(), Err(SolverError::RetryLimit));
    assert_eq!(
        controller
            .with_outcome(Outcome::Committed(indicators([0.0; 2])))
            .unwrap_err(),
        SolverError::RetryLimit
    );
}

#[test]
fn rejected_and_refused_attempts_remain_terminal_after_copying() {
    let initial = Controller::new(clock(), configuration()).unwrap();
    let started = initial
        .with_outcome(Outcome::Committed(indicators([0.5; 2])))
        .unwrap();
    for (outcome, reason) in [
        (
            Outcome::Rejected(indicators([1.01, 0.0])),
            StopReason::LocalErrorRejected,
        ),
        (
            Outcome::Rejected(indicators([0.0, 1.01])),
            StopReason::LocalErrorRejected,
        ),
        (
            Outcome::Refused {
                cause: SolverError::ProviderBudgetExceeded,
                indicators: None,
            },
            StopReason::Refused(SolverError::ProviderBudgetExceeded),
        ),
        (
            Outcome::Refused {
                cause: SolverError::InvalidPayload,
                indicators: Some(indicators([0.5; 2])),
            },
            StopReason::Refused(SolverError::InvalidPayload),
        ),
    ] {
        let stopped = started.with_outcome(outcome).unwrap();
        let restored = stopped;
        assert_eq!(restored.clock(), started.clock());
        assert_eq!(restored.attempted(), 2);
        assert_eq!(restored.committed(), 1);
        assert_eq!(restored.attempts_left(), 2);
        assert_eq!(restored.stopped(), Some(reason));
        assert_eq!(restored.next_clock(), Err(SolverError::RetryLimit));
    }
    assert_eq!(started.stopped(), None);
    assert_eq!(initial.clock(), clock());
}

#[test]
fn malformed_outcomes_do_not_change_controller_state() {
    let initial = Controller::new(clock(), configuration()).unwrap();
    for ratios in [[1.01, 0.0], [0.0, 1.01]] {
        assert_eq!(
            initial
                .with_outcome(Outcome::Committed(indicators(ratios)))
                .unwrap_err(),
            SolverError::InvalidPayload
        );
    }
    assert_eq!(
        initial
            .with_outcome(Outcome::Rejected(indicators([1.0, 0.0])))
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    for invalid in [-1.0, f64::NAN, f64::INFINITY] {
        for index in 0..4 {
            let mut bad = indicators([0.5; 2]);
            if index < 2 {
                bad.errors[index] = invalid;
            } else {
                bad.ratios[index - 2] = invalid;
            }
            for outcome in [
                Outcome::Committed(bad),
                Outcome::Rejected(bad),
                Outcome::Refused {
                    cause: SolverError::ResourceLimit,
                    indicators: Some(bad),
                },
            ] {
                assert_eq!(
                    initial.with_outcome(outcome).unwrap_err(),
                    SolverError::InvalidPayload
                );
            }
        }
    }
    assert_eq!(initial.attempted(), 0);
    assert_eq!(initial.clock(), clock());
    assert_eq!(
        Controller::new(clock().stages(4).unwrap()[4], configuration()).unwrap_err(),
        SolverError::InvalidClock
    );
    let mut invalid = configuration();
    invalid.tolerances.absolute[0] = 0.0;
    assert_eq!(
        Controller::new(clock(), invalid).unwrap_err(),
        SolverError::InvalidStep
    );
    invalid = configuration();
    invalid.limits.maximum_attempts = 2;
    assert_eq!(
        Controller::new(clock(), invalid).unwrap_err(),
        SolverError::RetryLimit
    );
}

#[test]
fn outcome_access_preserves_both_available_channels_and_missing_indicators() {
    let measured = indicators([0.25, 1.25]);
    for outcome in [
        Outcome::Committed(measured),
        Outcome::Rejected(measured),
        Outcome::Refused {
            cause: SolverError::ResourceLimit,
            indicators: Some(measured),
        },
    ] {
        assert_eq!(outcome.indicators(), Some(measured));
    }
    assert_eq!(
        Outcome::Refused {
            cause: SolverError::InvalidStep,
            indicators: None
        }
        .indicators(),
        None
    );
}
