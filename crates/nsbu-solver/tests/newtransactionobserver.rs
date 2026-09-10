//! Public recorded-step coverage for the observer transaction lifecycle.
use nsbu_solver::{
    diagnostics::balances::BalanceSample,
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        observer::{BalanceObserver, ObserverBounds},
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        kernel::{RhsBounds, RightHandSide},
        method::Method,
        trajectory::RunLimits,
        transaction::CandidateState,
    },
    Complex64, SolverError,
};

struct ZeroSource;
impl RightHandSide for ZeroSource {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }

    fn evaluate(
        &mut self,
        _: [&[Complex64]; 3],
        _: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for component in output {
            component.fill(Complex64::new(0.0, 0.0));
        }
        Ok(())
    }
}

struct PendingObserver {
    measured: usize,
    committed: usize,
    pending: bool,
}
impl BalanceObserver for PendingObserver {
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
        Ok(BalanceSample::REST)
    }

    fn commit_pending(&mut self) {
        assert!(self.pending);
        self.committed += 1;
        self.pending = false;
    }
}

#[test]
fn accepted_recorded_step_commits_the_staged_observer_state_once() {
    let configuration = Configuration {
        limits: RunLimits {
            endpoint: 4,
            step_ticks: 4,
            maximum_attempts: 1,
        },
        method: Method::CoxMatthews,
        tolerances: Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        },
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let workspace_bytes =
        AttemptWorkspace::reservation_with_method(domain, configuration.method).unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: workspace_bytes + std::mem::size_of::<PendingObserver>(),
            overhead: RunHistory::reservation(configuration).unwrap(),
        },
        1 << 20,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-8, 64).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new_with_method(plan, configuration.method).unwrap();
    let mut history = RunHistory::new(
        clock,
        configuration,
        RunHistory::reservation(configuration).unwrap(),
    )
    .unwrap();
    let mut observer = PendingObserver {
        measured: 0,
        committed: 0,
        pending: false,
    };
    assert!(matches!(
        recorded_step(
            &mut state,
            &mut candidate,
            &mut workspace,
            &mut ZeroSource,
            &mut observer,
            &mut history,
        )
        .unwrap(),
        Outcome::Committed(_)
    ));
    assert_eq!(state.accepted_steps(), 1);
    assert_eq!(history.balance().samples(), 2);
    assert_eq!(observer.measured, 1);
    assert_eq!(observer.committed, 1);
    assert!(!observer.pending);
}
