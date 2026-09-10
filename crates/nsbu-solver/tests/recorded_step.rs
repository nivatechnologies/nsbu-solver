//! Actual bounded runs retain every attempted outcome and refuse mismatched restart components.
mod mean_balance;
mod mean_source;
mod recorded_config;
mod recorded_support;
mod source_contract;
use nsbu_solver::{
    experiment::{control::Outcome, log::RunHistory, observer::ObserverBounds},
    integrators::{attempt::AttemptWorkspace, method::Method, trajectory::StopReason},
    SolverError,
};
use recorded_support::{configuration, source, Fixture, Observer};

#[test]
fn both_methods_retain_exact_attempts_and_complete_balance_history() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut config = configuration();
        config.method = method;
        let mut fixture = Fixture::new(config);
        let mut observer = Observer::default();
        let mut source = source(usize::MAX);
        for count in 1..=2 {
            assert!(matches!(
                fixture.step(&mut source, &mut observer).unwrap(),
                Outcome::Committed(_)
            ));
            let control = fixture.history.controller();
            assert_eq!(control.attempted(), count);
            assert_eq!(control.committed(), count);
            assert_eq!(control.clock(), fixture.state.clock());
            assert_eq!(observer.last_clock, Some(fixture.state.clock()));
            assert_eq!(fixture.history.balance().samples(), count + 1);
            assert_eq!(fixture.history.records().len(), count);
            assert_eq!(
                fixture.history.records()[count - 1].start.elapsed(),
                4 * (count - 1) as u128
            );
            assert!(matches!(
                fixture.history.records()[count - 1].outcome,
                Outcome::Committed(_)
            ));
        }
        assert_eq!(observer.calls, 2);
        assert_eq!(
            fixture.history.controller().stopped(),
            Some(StopReason::EndpointReached)
        );
        assert!(fixture
            .history
            .balance()
            .integral()
            .unwrap()
            .energy_defect
            .is_finite());
        assert_eq!(
            fixture.step(&mut source, &mut observer).unwrap_err(),
            SolverError::RetryLimit
        );
        assert_eq!(fixture.history.records().len(), 2);
    }
}

#[test]
fn integration_and_diagnostic_failures_are_recorded_without_advancing() {
    for case in 0..4 {
        let mut config = configuration();
        if case == 1 {
            config.tolerances.absolute = [1e-40; 2];
        }
        let mut fixture = Fixture::new(config);
        let mut observer = Observer::default();
        let mut source = source(if case == 0 { 1 } else { usize::MAX });
        if case == 2 {
            observer.fail_at = 1;
        }
        if case == 3 {
            observer.invalid_at = 1;
        }
        let outcome = fixture.step(&mut source, &mut observer).unwrap();
        match case {
            0 => assert_eq!(
                outcome,
                Outcome::Refused {
                    cause: SolverError::ProviderBudgetExceeded,
                    indicators: None
                }
            ),
            1 => {
                assert!(matches!(outcome, Outcome::Rejected(_)));
                assert!(outcome.indicators().unwrap().ratios[0] > 1.0);
            }
            _ => {
                let cause = if case == 2 {
                    SolverError::ResourceLimit
                } else {
                    SolverError::InvalidPayload
                };
                let indicators = outcome
                    .indicators()
                    .expect("diagnostic refusal retains completed indicators");
                assert_eq!(
                    outcome,
                    Outcome::Refused {
                        cause,
                        indicators: Some(indicators)
                    }
                );
            }
        }
        assert_eq!(fixture.state.clock().elapsed(), 0);
        assert_eq!(fixture.state.accepted_steps(), 0);
        assert_eq!(fixture.history.records().len(), 1);
        assert_eq!(fixture.history.controller().attempted(), 1);
        assert_eq!(fixture.history.balance().samples(), 1);
        assert_eq!(observer.calls, usize::from(case >= 2));
        assert_eq!(
            fixture.step(&mut source, &mut observer).unwrap_err(),
            SolverError::RetryLimit
        );
    }
}

#[test]
fn later_failure_preserves_earlier_commits_and_pending_diagnostics() {
    let mut fixture = Fixture::new(configuration());
    let mut observer = Observer {
        fail_at: 2,
        ..Observer::default()
    };
    let mut source = source(usize::MAX);
    fixture.step(&mut source, &mut observer).unwrap();
    let energy = fixture.history.balance();
    fixture.step(&mut source, &mut observer).unwrap();
    assert_eq!(fixture.state.clock().elapsed(), 4);
    assert_eq!(fixture.history.controller().committed(), 1);
    assert_eq!(fixture.history.controller().attempted(), 2);
    assert_eq!(fixture.history.records()[1].start.elapsed(), 4);
    assert_eq!(fixture.history.balance().samples(), energy.samples());
    assert_eq!(fixture.history.balance().clock(), energy.clock());
    assert!(fixture.history.balance().has_pending_midpoint());
}

#[test]
fn cost_preflight_refuses_before_any_attempt() {
    let bytes = RunHistory::reservation(configuration()).unwrap();
    let mut exact = Fixture::with_overhead(configuration(), bytes);
    assert!(matches!(
        exact
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap(),
        Outcome::Committed(_)
    ));
    for (bounds, error) in [
        (None, SolverError::UnknownProviderCost),
        (
            Some(ObserverBounds {
                storage_bytes: usize::MAX,
                work_units: 1,
            }),
            SolverError::SizeOverflow,
        ),
        (
            Some(ObserverBounds {
                storage_bytes: 1 << 20,
                work_units: 1,
            }),
            SolverError::ResourceLimit,
        ),
        (
            Some(ObserverBounds {
                storage_bytes: 0,
                work_units: 0,
            }),
            SolverError::ResourceLimit,
        ),
        (
            Some(ObserverBounds {
                storage_bytes: 0,
                work_units: usize::MAX,
            }),
            SolverError::SizeOverflow,
        ),
    ] {
        let mut fixture = Fixture::new(configuration());
        let mut observer = Observer {
            bounds,
            ..Observer::default()
        };
        assert_eq!(
            fixture
                .step(&mut source(usize::MAX), &mut observer)
                .unwrap_err(),
            error
        );
        assert_eq!(fixture.state.accepted_steps(), 0);
        assert!(fixture.history.records().is_empty());
        assert_eq!(observer.calls, 0);
    }
    let mut config = configuration();
    config.limits.maximum_attempts = 1000;
    let mut fixture = Fixture::new(config);
    assert_eq!(
        fixture
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap_err(),
        SolverError::ResourceLimit
    );
    assert!(fixture.history.records().is_empty());
}

#[test]
fn mismatched_clock_count_and_method_cannot_resume_a_history() {
    let mut config = configuration();
    config.limits.endpoint = 12;
    let mut wrong_clock = Fixture::new(config);
    wrong_clock.bare(4);
    assert_eq!(
        wrong_clock
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    let mut recorded = Fixture::new(config);
    for _ in 0..2 {
        recorded
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap();
    }
    let mut different_count = Fixture::new(config);
    different_count.bare(8);
    different_count.history = recorded.history;
    assert_eq!(
        different_count
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    config.method = Method::HochbruckOstermann;
    let mut different_method = Fixture::new(config);
    different_method.workspace =
        AttemptWorkspace::new_with_method(different_method.state.plan(), Method::CoxMatthews)
            .unwrap();
    assert_eq!(
        different_method
            .step(&mut source(usize::MAX), &mut Observer::default())
            .unwrap_err(),
        SolverError::InvalidPayload
    );
}

#[test]
fn history_preflight_and_one_step_pending_state_are_explicit() {
    let fixture = Fixture::new(configuration());
    let clock = fixture.state.clock();
    assert_eq!(
        mean_balance::measured(&fixture.state),
        nsbu_solver::diagnostics::balances::BalanceSample::REST
    );
    let bytes = RunHistory::reservation(configuration()).unwrap();
    assert_eq!(
        RunHistory::new(clock, configuration(), bytes - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let mut config = configuration();
    config.limits.maximum_attempts = usize::MAX;
    assert_eq!(
        RunHistory::reservation(config),
        Err(SolverError::SizeOverflow)
    );
    config = configuration();
    config.limits.endpoint = 4;
    let mut one = Fixture::new(config);
    one.step(&mut source(usize::MAX), &mut Observer::default())
        .unwrap();
    assert_eq!(
        one.history.controller().stopped(),
        Some(StopReason::EndpointReached)
    );
    assert!(one.history.balance().has_pending_midpoint());
    assert_eq!(
        one.history.balance().integral().unwrap_err(),
        SolverError::InvalidPayload
    );
}
