//! Replay retains failures and validates bounded log geometry without asserting physical provenance.
mod recorded_config;
use nsbu_solver::{
    diagnostics::balances::BalanceSample,
    domain::TickClock,
    experiment::{
        control::Outcome,
        log::{AttemptRecord, RunHistory},
    },
    integrators::{indicator::Indicators, trajectory::StopReason},
    SolverError,
};

fn record(clock: TickClock, outcome: Outcome, sample: Option<BalanceSample>) -> AttemptRecord {
    AttemptRecord {
        start: clock,
        outcome,
        sample,
    }
}
fn indicators() -> Indicators {
    Indicators {
        errors: [0.0; 2],
        ratios: [0.0; 2],
    }
}

#[test]
fn replay_keeps_raw_samples_pending_quadrature_and_terminal_failures() {
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let config = recorded_config::configuration();
    let first = record(
        clock,
        Outcome::Committed(indicators()),
        Some(BalanceSample::REST),
    );
    let next = clock.stages(4).unwrap()[4];
    let refused = Outcome::Refused {
        cause: SolverError::ProviderBudgetExceeded,
        indicators: None,
    };
    let records = [first, record(next, refused, None)];
    let history = RunHistory::replay(clock, config, &records, usize::MAX).unwrap();
    assert_eq!(history.controller().clock(), next);
    assert_eq!(history.controller().attempted(), 2);
    assert_eq!(history.controller().committed(), 1);
    assert_eq!(
        history.controller().stopped(),
        Some(StopReason::Refused(SolverError::ProviderBudgetExceeded))
    );
    assert_eq!(history.records()[0].sample, Some(BalanceSample::REST));
    assert_eq!(history.records()[1].sample, None);
    assert_eq!(history.records()[1].outcome, refused);
    assert_eq!(history.balance().samples(), 2);
    assert!(history.balance().has_pending_midpoint());
    assert_eq!(
        history.balance().integral().unwrap_err(),
        SolverError::InvalidPayload
    );
    let empty = RunHistory::replay(clock, config, &[], usize::MAX).unwrap();
    assert_eq!(empty.controller().attempted(), 0);
    assert!(empty.records().is_empty());
}

#[test]
fn malformed_replay_refuses_counts_clock_sample_and_post_terminal_records() {
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let config = recorded_config::configuration();
    let good = record(
        clock,
        Outcome::Committed(indicators()),
        Some(BalanceSample::REST),
    );
    assert_eq!(
        RunHistory::replay(clock, config, &[good; 4], usize::MAX).unwrap_err(),
        SolverError::ResourceLimit
    );
    assert_eq!(
        RunHistory::replay(clock, config, &[], 0).unwrap_err(),
        SolverError::ResourceLimit
    );
    let next = clock.stages(4).unwrap()[4];
    let wrong_clock = record(next, good.outcome, good.sample);
    assert_eq!(
        RunHistory::replay(clock, config, &[wrong_clock], usize::MAX).unwrap_err(),
        SolverError::InvalidClock
    );
    let missing = record(clock, good.outcome, None);
    assert_eq!(
        RunHistory::replay(clock, config, &[missing], usize::MAX).unwrap_err(),
        SolverError::InvalidPayload
    );
    let failure = record(
        clock,
        Outcome::Refused {
            cause: SolverError::AdvectiveLimit,
            indicators: None,
        },
        None,
    );
    assert_eq!(
        RunHistory::replay(clock, config, &[failure, good], usize::MAX).unwrap_err(),
        SolverError::RetryLimit
    );
    let mut config = config;
    config.limits.maximum_attempts = 2;
    let terminal = [good, record(next, good.outcome, good.sample)];
    let history = RunHistory::replay(
        clock,
        config,
        &terminal,
        RunHistory::reservation(config).unwrap(),
    )
    .unwrap();
    assert_eq!(
        history.controller().stopped(),
        Some(StopReason::EndpointReached)
    );
    assert_eq!(
        history
            .balance()
            .integral()
            .unwrap()
            .energy_defect
            .to_bits(),
        0.0_f64.to_bits()
    );
}

#[test]
fn complete_raw_log_replays_cancellation_in_the_original_order() {
    let clock = TickClock::from_rest(-2, 100).unwrap();
    let mut config = recorded_config::configuration();
    config.limits.endpoint = 24;
    config.limits.maximum_attempts = 6;
    let mut start = clock;
    let records = [7.5e15, 0.0, 0.75, 0.0, -7.5e15, 0.0].map(|work| {
        let sample = BalanceSample {
            forcing_work: work,
            stretching: work,
            vorticity_forcing: work,
            ..BalanceSample::REST
        };
        let entry = record(start, Outcome::Committed(indicators()), Some(sample));
        start = start.stages(4).unwrap()[4];
        entry
    });
    let partial = RunHistory::replay(clock, config, &records[..5], usize::MAX).unwrap();
    assert!(partial.balance().has_pending_midpoint());
    let complete = RunHistory::replay(clock, config, &records, usize::MAX).unwrap();
    let integral = complete.balance().integral().unwrap();
    assert_eq!(integral.energy_rhs.to_bits(), 1.0_f64.to_bits());
    assert_eq!(integral.enstrophy_rhs.to_bits(), 2.0_f64.to_bits());
    assert_eq!(integral.energy_defect.to_bits(), (-1.0_f64).to_bits());
    assert_eq!(integral.enstrophy_defect.to_bits(), (-2.0_f64).to_bits());
}
