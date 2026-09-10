//! Bounded trajectory progress, rejection and partial-failure evidence.
mod transaction_support;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        kernel::{RhsBounds, RightHandSide},
        trajectory::{FixedRun, RunLimits, StopReason},
    },
    Complex64, SolverError,
};

struct Source {
    calls: usize,
    fail_at: usize,
}
impl RightHandSide for Source {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
            values[0] = Complex64::new((clock.elapsed() as f64).cos(), 0.0);
        }
        Ok(())
    }
}
fn plan() -> ResourcePlan {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 4096,
        },
        1 << 20,
        Epoch(0),
    )
    .unwrap()
}
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
        let (mut state, mut candidate, mut workspace) =
            transaction_support::setup(plan(), TickClock::from_rest(-12, 64).unwrap());
        let mut source = Source { calls: 0, fail_at };
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
    let (mut state, mut candidate, mut workspace) =
        transaction_support::setup(plan(), TickClock::from_rest(-12, 64).unwrap());
    let mut source = Source {
        calls: 0,
        fail_at: usize::MAX,
    };
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
