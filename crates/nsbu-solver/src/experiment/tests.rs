//! Malformed private holders cannot publish accepted history or bypass a stale token.
use super::{
    control::Outcome,
    log::RunHistory,
    observer::{BalanceObserver, ObserverBounds},
    runner::completed,
};
use crate::{
    diagnostics::balances::BalanceSample,
    domain::{Epoch, SpectralState},
    integrators::{attempt::AttemptResult, indicator::Indicators},
    test_support::transaction_setup,
    SolverError,
};

fn history() -> RunHistory {
    let (_, state, _) = transaction_setup();
    let config = crate::test_support::configuration();
    RunHistory::new(
        state.clock(),
        config,
        RunHistory::reservation(config).unwrap(),
    )
    .unwrap()
}
fn indicators() -> Indicators {
    Indicators {
        errors: [0.0; 2],
        ratios: [0.0; 2],
    }
}

#[test]
fn private_update_admission_refuses_missing_or_misclassified_samples() {
    let history = history();
    assert!(matches!(
        history.prepare(Outcome::Committed(indicators()), None),
        Err(SolverError::InvalidPayload)
    ));
    assert!(matches!(
        history.prepare(
            Outcome::Refused {
                cause: SolverError::ResourceLimit,
                indicators: None
            },
            Some(BalanceSample::REST)
        ),
        Err(SolverError::InvalidPayload)
    ));
    let invalid = BalanceSample {
        forcing_work: f64::NAN,
        ..BalanceSample::REST
    };
    assert!(matches!(
        history.prepare(Outcome::Committed(indicators()), Some(invalid)),
        Err(SolverError::InvalidPayload)
    ));
    assert_eq!(history.records().len(), 0);
    assert_eq!(history.controller().attempted(), 0);
}

struct TestObserver {
    observed: bool,
    refuses: bool,
}

#[derive(Clone, Copy)]
enum TransactionMode {
    Accept,
    FailMeasure,
    InvalidSample,
}

struct TransactionObserver {
    mode: TransactionMode,
    measured: usize,
    committed: usize,
    discarded: usize,
    pending: bool,
}
impl TransactionObserver {
    fn new(mode: TransactionMode) -> Self {
        Self {
            mode,
            measured: 0,
            committed: 0,
            discarded: 0,
            pending: false,
        }
    }
}
impl BalanceObserver for TransactionObserver {
    fn bounds(&self) -> Option<ObserverBounds> {
        Some(ObserverBounds {
            storage_bytes: std::mem::size_of::<Self>(),
            work_units: 1,
        })
    }

    fn measure(&mut self, _: &SpectralState) -> Result<BalanceSample, SolverError> {
        assert!(!self.pending);
        self.measured += 1;
        self.pending = true;
        match self.mode {
            TransactionMode::Accept => Ok(BalanceSample::REST),
            TransactionMode::FailMeasure => Err(SolverError::ResourceLimit),
            TransactionMode::InvalidSample => Ok(BalanceSample {
                forcing_work: f64::NAN,
                ..BalanceSample::REST
            }),
        }
    }

    fn commit_pending(&mut self) {
        assert!(self.pending);
        self.committed += 1;
        self.pending = false;
    }

    fn discard_pending(&mut self) {
        assert!(self.pending);
        self.discarded += 1;
        self.pending = false;
    }
}

#[test]
fn observer_pending_state_tracks_the_physical_and_history_transaction() {
    for case in 0..5 {
        let (_, mut state, mut candidate) = transaction_setup();
        let endpoint = state.clock().stages(4).unwrap()[4];
        candidate.state.clock = endpoint;
        candidate.state.accepted_steps = 1;
        candidate.state.epoch = Epoch(1);
        let accepted = candidate.accept(&state);
        let (result, mode) = match case {
            0 => (
                AttemptResult {
                    indicators: indicators(),
                    ticks: 4,
                    rhs_calls: 12,
                    accepted: Some(accepted),
                },
                TransactionMode::Accept,
            ),
            1 => {
                let mut rejected = indicators();
                rejected.ratios[0] = 2.0;
                (
                    AttemptResult {
                        indicators: rejected,
                        ticks: 4,
                        rhs_calls: 12,
                        accepted: None,
                    },
                    TransactionMode::Accept,
                )
            }
            2 => (
                AttemptResult {
                    indicators: indicators(),
                    ticks: 4,
                    rhs_calls: 12,
                    accepted: Some(accepted),
                },
                TransactionMode::FailMeasure,
            ),
            3 => (
                AttemptResult {
                    indicators: indicators(),
                    ticks: 4,
                    rhs_calls: 12,
                    accepted: Some(accepted),
                },
                TransactionMode::InvalidSample,
            ),
            _ => {
                candidate.invalidate().unwrap();
                (
                    AttemptResult {
                        indicators: indicators(),
                        ticks: 4,
                        rhs_calls: 12,
                        accepted: Some(accepted),
                    },
                    TransactionMode::Accept,
                )
            }
        };
        let mut observer = TransactionObserver::new(mode);
        let mut history = history();
        let outcome = completed(
            &mut state,
            &mut candidate,
            &mut observer,
            &mut history,
            result,
            endpoint,
        )
        .unwrap();
        assert_eq!(state.accepted_steps(), u128::from(case == 0));
        assert_eq!(history.balance().samples(), 1 + usize::from(case == 0));
        assert_eq!(
            observer.measured,
            usize::from((case == 0) || (2..=3).contains(&case))
        );
        assert_eq!(observer.committed, usize::from(case == 0));
        assert_eq!(observer.discarded, usize::from((2..=3).contains(&case)));
        assert!(!observer.pending);
        if case == 1 {
            assert!(matches!(outcome, Outcome::Rejected(_)));
        } else if case == 0 {
            assert_eq!(outcome, Outcome::Committed(indicators()));
        } else {
            assert!(matches!(outcome, Outcome::Refused { .. }));
        }
    }
}

#[test]
fn failed_refusal_recording_still_discards_a_measured_observer_proposal() {
    let (_, mut state, mut candidate) = transaction_setup();
    let endpoint = state.clock().stages(4).unwrap()[4];
    candidate.state.clock = endpoint;
    candidate.state.accepted_steps = 1;
    candidate.state.epoch = Epoch(1);
    let accepted = candidate.accept(&state);
    let mut malformed = indicators();
    malformed.errors[0] = f64::NAN;
    let result = AttemptResult {
        indicators: malformed,
        ticks: 4,
        rhs_calls: 12,
        accepted: Some(accepted),
    };
    let mut observer = TransactionObserver::new(TransactionMode::FailMeasure);
    let mut history = history();
    assert_eq!(
        completed(
            &mut state,
            &mut candidate,
            &mut observer,
            &mut history,
            result,
            endpoint,
        ),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(state.accepted_steps(), 0);
    assert!(history.records().is_empty());
    assert_eq!(observer.measured, 1);
    assert_eq!(observer.discarded, 1);
    assert!(!observer.pending);
}
impl BalanceObserver for TestObserver {
    fn bounds(&self) -> Option<ObserverBounds> {
        None
    }
    fn measure(&mut self, _: &SpectralState) -> Result<BalanceSample, SolverError> {
        self.observed = true;
        if self.refuses {
            Err(SolverError::ResourceLimit)
        } else {
            Ok(BalanceSample::REST)
        }
    }
}
#[test]
fn injected_stale_or_inconsistent_holders_cannot_publish_accepted_history() {
    for case in 0..5 {
        let (_, mut state, mut candidate) = transaction_setup();
        let endpoint = state.clock().stages(4).unwrap()[4];
        candidate.state.clock = if case == 1 { state.clock() } else { endpoint };
        candidate.state.accepted_steps = u128::from(case != 2);
        candidate.state.epoch = Epoch(1);
        let token = candidate.accept(&state);
        if case == 0 {
            candidate.invalidate().unwrap();
        }
        let mut observer = TestObserver {
            observed: false,
            refuses: case == 3,
        };
        assert!(observer.bounds().is_none());
        let result = AttemptResult {
            indicators: indicators(),
            ticks: 4,
            rhs_calls: 12,
            accepted: Some(token),
        };
        let mut history = history();
        let outcome = completed(
            &mut state,
            &mut candidate,
            &mut observer,
            &mut history,
            result,
            endpoint,
        )
        .unwrap();
        let expected = match case {
            0 => Outcome::Refused {
                cause: SolverError::StaleAttempt,
                indicators: Some(indicators()),
            },
            1 | 2 => Outcome::Refused {
                cause: SolverError::InvalidPayload,
                indicators: Some(indicators()),
            },
            3 => Outcome::Refused {
                cause: SolverError::ResourceLimit,
                indicators: Some(indicators()),
            },
            _ => Outcome::Committed(indicators()),
        };
        assert_eq!(outcome, expected);
        assert_eq!(observer.observed, case >= 3);
        assert_eq!(state.accepted_steps(), u128::from(case == 4));
        assert_eq!(history.records().len(), 1);
        assert_eq!(history.balance().samples(), 1 + usize::from(case == 4));
    }
}

#[test]
fn private_rejection_reports_must_actually_fail_a_local_channel() {
    for valid_rejection in [true, false] {
        let (_, mut state, mut candidate) = transaction_setup();
        let endpoint = state.clock().stages(4).unwrap()[4];
        let mut observer = TestObserver {
            observed: false,
            refuses: false,
        };
        let mut measured = indicators();
        measured.ratios[0] = if valid_rejection { 2.0 } else { 0.0 };
        let result = AttemptResult {
            indicators: measured,
            ticks: 4,
            rhs_calls: 12,
            accepted: None,
        };
        let mut history = history();
        let outcome = completed(
            &mut state,
            &mut candidate,
            &mut observer,
            &mut history,
            result,
            endpoint,
        );
        if valid_rejection {
            assert_eq!(outcome.unwrap(), Outcome::Rejected(measured));
        } else {
            assert_eq!(outcome.unwrap_err(), SolverError::InvalidPayload);
        }
        assert!(!observer.observed);
        assert_eq!(state.accepted_steps(), 0);
        assert_eq!(history.records().len(), usize::from(valid_rejection));
    }
}
