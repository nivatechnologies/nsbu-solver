//! Raw history serialization preserves all channels and refuses malformed bounded imports.
mod mean_balance;
mod mean_source;
mod recorded_config;
mod recorded_support;
mod source_contract;
use nsbu_solver::checkpoint::{
    history::{encoded_len, read, reservation, write},
    CheckpointError,
};
use nsbu_solver::diagnostics::{balances::BalanceSample, norms::Norms};
use nsbu_solver::domain::TickClock;
use nsbu_solver::experiment::{
    control::Outcome,
    log::{AttemptRecord, RunHistory},
};
use nsbu_solver::integrators::{indicator::Indicators, method::Method};
use nsbu_solver::SolverError;

fn clock() -> TickClock {
    TickClock::from_rest(-12, 100).unwrap()
}
fn indicators() -> Indicators {
    Indicators {
        errors: [0.25, 0.125],
        ratios: [0.5, 1.0],
    }
}
fn sample() -> BalanceSample {
    BalanceSample {
        norms: Norms {
            l2: 1.0,
            h1: 2.0,
            vorticity_l2: 3.0,
            divergence_l2: 0.0,
        },
        energy: 0.5,
        enstrophy: 4.5,
        energy_dissipation: 0.125,
        forcing_work: -0.25,
        stretching: 0.0625,
        enstrophy_dissipation: 0.03125,
        vorticity_forcing: -0.0,
    }
}
fn encoded(history: &RunHistory) -> Vec<u8> {
    let required = encoded_len(history).unwrap();
    assert_eq!(required, 157 + 176 * history.records().len());
    let mut bytes = vec![0x55; required + 1];
    assert_eq!(
        write(history, &mut bytes[..required - 1]),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(bytes.iter().all(|b| *b == 0x55));
    assert_eq!(write(history, &mut bytes), Ok(required));
    assert_eq!(bytes[required], 0x55);
    bytes.truncate(required);
    bytes
}
fn one(outcome: Outcome, sample: Option<BalanceSample>) -> RunHistory {
    let records = [AttemptRecord {
        start: clock(),
        outcome,
        sample,
    }];
    RunHistory::replay(
        clock(),
        recorded_config::configuration(),
        &records,
        usize::MAX,
    )
    .unwrap()
}

#[test]
fn committed_logs_preserve_both_methods_configuration_raw_samples_and_pending_pairs() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut config = recorded_config::configuration();
        config.method = method;
        config.tolerances.relative = [0.03125, 0.0625];
        let records = [
            AttemptRecord {
                start: clock(),
                outcome: Outcome::Committed(indicators()),
                sample: Some(sample()),
            },
            AttemptRecord {
                start: clock().stages(4).unwrap()[4],
                outcome: Outcome::Committed(indicators()),
                sample: Some(sample()),
            },
        ];
        for count in 0..=2 {
            let original =
                RunHistory::replay(clock(), config, &records[..count], usize::MAX).unwrap();
            let bytes = encoded(&original);
            let cap = reservation(config, count).unwrap();
            let restored = read(&bytes, bytes.len(), config.limits.maximum_attempts, cap).unwrap();
            assert_eq!(restored.controller().clock(), original.controller().clock());
            assert_eq!(restored.controller().attempted(), count);
            assert_eq!(restored.controller().committed(), count);
            assert_eq!(
                restored.controller().stopped(),
                original.controller().stopped()
            );
            assert_eq!(restored.controller().configuration().method, method);
            assert_eq!(restored.balance().has_pending_midpoint(), count == 1);
            assert_eq!(encoded(&restored), bytes);
            for record in restored.records() {
                assert_eq!(record.outcome, Outcome::Committed(indicators()));
                assert_eq!(record.sample, Some(sample()));
                assert_eq!(
                    record.sample.unwrap().vorticity_forcing.to_bits(),
                    (-0.0_f64).to_bits()
                );
            }
            assert_eq!(
                read(&bytes, bytes.len(), 3, cap - 1).unwrap_err(),
                CheckpointError::ResourceLimit
            );
        }
    }
}

#[test]
fn rejection_and_every_failure_code_preserve_optional_indicators() {
    let errors = [
        SolverError::InvalidDomain,
        SolverError::InvalidIndex,
        SolverError::SizeOverflow,
        SolverError::InvalidClock,
        SolverError::InvalidStep,
        SolverError::ClockCapacityExceeded,
        SolverError::EpochExhausted,
        SolverError::ResourceLimit,
        SolverError::AllocationFailed,
        SolverError::InvalidPayload,
        SolverError::InvalidSpectrum,
        SolverError::ArithmeticResolutionLimited,
        SolverError::StaleAttempt,
        SolverError::RetryLimit,
        SolverError::UnknownProviderCost,
        SolverError::ProviderBudgetExceeded,
        SolverError::AdvectiveLimit,
    ];
    for (index, cause) in errors.into_iter().enumerate() {
        for indicators in [None, Some(indicators())] {
            let outcome = Outcome::Refused { cause, indicators };
            let bytes = encoded(&one(outcome, None));
            assert_eq!(bytes[210], (index + 1) as u8);
            let restored = read(&bytes, bytes.len(), 3, usize::MAX).unwrap();
            assert_eq!(restored.records()[0].outcome, outcome);
            assert_eq!(restored.records()[0].sample, None);
            assert_eq!(encoded(&restored), bytes);
        }
    }
    let outcome = Outcome::Rejected(Indicators {
        ratios: [1.25, 0.5],
        ..indicators()
    });
    let bytes = encoded(&one(outcome, None));
    assert_eq!(
        read(&bytes, bytes.len(), 3, usize::MAX).unwrap().records()[0].outcome,
        outcome
    );
}

#[test]
fn lengths_clock_configuration_and_storage_are_preflighted() {
    let bytes = encoded(&one(Outcome::Committed(indicators()), Some(sample())));
    assert_eq!(
        read(&bytes, bytes.len() - 1, 3, usize::MAX).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    assert_eq!(
        read(&bytes, bytes.len(), 2, usize::MAX).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    let mut excess_records = bytes.clone();
    excess_records[141..157].copy_from_slice(&4_u128.to_le_bytes());
    assert_eq!(
        read(&excess_records, excess_records.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    for count in 0..bytes.len() {
        assert!(read(&bytes[..count], bytes.len(), 3, usize::MAX).is_err());
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert_eq!(
        read(&bad, bad.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    for index in [0, 60] {
        let mut bad = bytes.clone();
        bad[index] = 255;
        assert_eq!(
            read(&bad, bad.len(), 3, usize::MAX).unwrap_err(),
            CheckpointError::InvalidEncoding
        );
    }
    for index in [93, 141] {
        let mut bad = bytes.clone();
        bad[index..index + 16].fill(255);
        assert_eq!(
            read(&bad, bad.len(), 3, usize::MAX).unwrap_err(),
            CheckpointError::ResourceLimit
        );
    }
    let mut invalid_clock = bytes.clone();
    invalid_clock[44..60].fill(0);
    assert_eq!(
        read(&invalid_clock, invalid_clock.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidHistory(SolverError::InvalidClock)
    );
    let mut invalid_step = bytes.clone();
    invalid_step[77..93].fill(0);
    assert_eq!(
        read(&invalid_step, invalid_step.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidHistory(SolverError::InvalidStep)
    );
}

#[test]
fn noncanonical_flags_fields_and_invalid_measurements_cannot_form_a_history() {
    let bytes = encoded(&one(Outcome::Committed(indicators()), Some(sample())));
    for (index, value) in [(209, 0), (210, 1), (211, 0), (211, 2), (244, 0), (244, 2)] {
        let mut bad = bytes.clone();
        bad[index] = value;
        assert_eq!(
            read(&bad, bad.len(), 3, usize::MAX).unwrap_err(),
            CheckpointError::InvalidEncoding
        );
    }
    let mut nan = bytes;
    nan[245..253].copy_from_slice(&f64::NAN.to_bits().to_le_bytes());
    assert_eq!(
        read(&nan, nan.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidHistory(SolverError::InvalidPayload)
    );
    let refused = encoded(&one(
        Outcome::Refused {
            cause: SolverError::InvalidDomain,
            indicators: None,
        },
        None,
    ));
    for cause in [0, 18, 255] {
        let mut bad = refused.clone();
        bad[210] = cause;
        assert_eq!(
            read(&bad, bad.len(), 3, usize::MAX).unwrap_err(),
            CheckpointError::InvalidEncoding
        );
    }
}

#[test]
fn actual_histories_resume_the_same_commit_and_failure_with_fresh_scratch() {
    use nsbu_solver::lineage::PhysicalImage;
    use recorded_support::{source, Fixture, Observer};
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        for fail_at in [1, usize::MAX] {
            let mut config = recorded_config::configuration();
            config.method = method;
            let mut original = Fixture::new(config);
            let mut observer = Observer::default();
            assert!(matches!(
                original
                    .step(&mut source(usize::MAX), &mut observer)
                    .unwrap(),
                Outcome::Committed(_)
            ));
            let bytes = encoded(&original.history);
            let plan = original.state.plan();
            let physical =
                PhysicalImage::capture(&original.state, PhysicalImage::reservation(plan).unwrap())
                    .unwrap();
            let mut resumed = Fixture::new(config);
            resumed.state = physical.into_state();
            resumed.history = read(&bytes, bytes.len(), 3, usize::MAX).unwrap();
            assert!(resumed.history.balance().has_pending_midpoint());
            let expected = original.step(&mut source(fail_at), &mut observer).unwrap();
            let actual = resumed
                .step(&mut source(fail_at), &mut Observer::default())
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(encoded(&resumed.history), encoded(&original.history));
            assert_eq!(resumed.state.clock(), original.state.clock());
            assert_eq!(
                resumed.state.accepted_steps(),
                original.state.accepted_steps()
            );
            assert_eq!(resumed.state.epoch(), original.state.epoch());
            for axis in 0..3 {
                for (left, right) in original
                    .state
                    .component(axis)
                    .unwrap()
                    .iter()
                    .zip(resumed.state.component(axis).unwrap())
                {
                    assert_eq!(
                        [left.re.to_bits(), left.im.to_bits()],
                        [right.re.to_bits(), right.im.to_bits()]
                    );
                }
            }
            if fail_at == 1 {
                assert!(matches!(
                    actual,
                    Outcome::Refused {
                        cause: SolverError::ProviderBudgetExceeded,
                        ..
                    }
                ));
                assert_eq!(resumed.history.balance().samples(), 2);
            } else {
                assert!(matches!(actual, Outcome::Committed(_)));
                let left = original.history.balance().integral().unwrap();
                let right = resumed.history.balance().integral().unwrap();
                assert_eq!(
                    [
                        left.energy_rhs,
                        left.enstrophy_rhs,
                        left.energy_defect,
                        left.enstrophy_defect
                    ]
                    .map(f64::to_bits),
                    [
                        right.energy_rhs,
                        right.enstrophy_rhs,
                        right.energy_defect,
                        right.enstrophy_defect
                    ]
                    .map(f64::to_bits)
                );
            }
        }
    }
}

#[test]
fn imported_history_does_not_authorize_a_different_physical_clock() {
    use recorded_support::{source, Fixture, Observer};
    let mut fixture = Fixture::new(recorded_config::configuration());
    let bytes = encoded(&fixture.history);
    fixture.bare(4);
    fixture.history = read(&bytes, bytes.len(), 3, usize::MAX).unwrap();
    let before = fixture.state.clock();
    let mut observer = Observer::default();
    assert_eq!(
        fixture.step(&mut source(usize::MAX), &mut observer),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(fixture.state.clock(), before);
    assert_eq!(observer.calls, 0);
    assert!(fixture.history.records().is_empty());
}

#[test]
fn canonical_bytes_still_require_semantically_consistent_attempts() {
    let original = encoded(&one(Outcome::Committed(indicators()), Some(sample())));
    let mut missing_sample = original.clone();
    missing_sample[244..333].fill(0);
    assert_eq!(
        read(&missing_sample, missing_sample.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidHistory(SolverError::InvalidPayload)
    );
    for offset in [212, 228, 245, 277] {
        let mut negative = original.clone();
        negative[offset..offset + 8].copy_from_slice(&(-1.0_f64).to_bits().to_le_bytes());
        assert_eq!(
            read(&negative, negative.len(), 3, usize::MAX).unwrap_err(),
            CheckpointError::InvalidHistory(SolverError::InvalidPayload)
        );
    }
    let mut wrong_start = original;
    wrong_start[177..193].copy_from_slice(&4_u128.to_le_bytes());
    wrong_start[193..209].copy_from_slice(&96_u128.to_le_bytes());
    assert_eq!(
        read(&wrong_start, wrong_start.len(), 3, usize::MAX).unwrap_err(),
        CheckpointError::InvalidHistory(SolverError::InvalidClock)
    );
}
