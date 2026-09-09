//! Admission and arithmetic refusals before transactional state mutation.
mod transaction_support;
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::kernel::{RhsBounds, RightHandSide};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace, indicator::Tolerances, transaction::CandidateState,
};
use nsbu_solver::{Complex64, SolverError};

struct Source {
    bounds: Option<RhsBounds>,
    value: f64,
    index: usize,
    refuse: bool,
}
impl RightHandSide for Source {
    fn bounds(&self) -> Option<RhsBounds> {
        self.bounds
    }
    fn begin_attempt(&mut self, _clock: TickClock, _ticks: u128) -> Result<(), SolverError> {
        if self.refuse {
            Err(SolverError::ProviderBudgetExceeded)
        } else {
            Ok(())
        }
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
            values[self.index] = Complex64::new(self.value, 0.0);
        }
        Ok(())
    }
}
fn plan(epoch: Epoch, diagnostics: usize) -> ResourcePlan {
    ResourcePlan::new(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        ExtraStorage {
            fft: 0,
            force: 128,
            diagnostics,
            overhead: 4096,
        },
        1024 * 1024,
        epoch,
    )
    .unwrap()
}
fn bounds() -> RhsBounds {
    RhsBounds {
        storage_bytes: 128,
        work_units: 144,
        scalar_transforms: 0,
    }
}

#[test]
fn workspace_reservations_must_be_complete_and_addressable() {
    assert!(matches!(
        AttemptWorkspace::new(plan(Epoch(0), 0)),
        Err(SolverError::ResourceLimit)
    ));
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let expected = nsbu_solver::integrators::kernel::CmWorkspace::reservation(48).unwrap()
        + 48 * (6 * std::mem::size_of::<Complex64>()
            + 2 * std::mem::size_of::<nsbu_solver::integrators::coefficients::CmCoefficients>())
        + std::mem::size_of::<AttemptWorkspace>();
    assert_eq!(AttemptWorkspace::reservation(domain), Ok(expected));
    let kernel_overflow = Domain::new([1 << 24, 1_600_000_000, 4], [1.0; 3], 1.0).unwrap();
    assert_eq!(
        AttemptWorkspace::reservation(kernel_overflow),
        Err(SolverError::SizeOverflow)
    );
    let large = Domain::new([1 << 31, 1 << 24, 4], [1.0; 3], 1.0).unwrap();
    assert_eq!(
        AttemptWorkspace::reservation(large),
        Err(SolverError::SizeOverflow)
    );
}

#[test]
fn invalid_policy_and_unadmitted_rhs_costs_preserve_the_clock() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let plan = plan(Epoch(0), AttemptWorkspace::reservation(domain).unwrap());
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let (state, mut candidate, mut work) = transaction_support::setup(plan, clock);
    let tolerance = Tolerances {
        absolute: [1e-10; 2],
        relative: [1e-7; 2],
    };
    for (declaration, expected) in [
        (None, SolverError::UnknownProviderCost),
        (
            Some(RhsBounds {
                storage_bytes: 129,
                ..bounds()
            }),
            SolverError::ResourceLimit,
        ),
        (
            Some(RhsBounds {
                work_units: 0,
                ..bounds()
            }),
            SolverError::ResourceLimit,
        ),
        (
            Some(RhsBounds {
                work_units: usize::MAX,
                ..bounds()
            }),
            SolverError::SizeOverflow,
        ),
        (
            Some(RhsBounds {
                scalar_transforms: usize::MAX,
                ..bounds()
            }),
            SolverError::SizeOverflow,
        ),
    ] {
        let mut rhs = Source {
            bounds: declaration,
            value: 0.0,
            index: 0,
            refuse: false,
        };
        assert_eq!(
            (work.try_advance(&state, &mut candidate, 8, tolerance, &mut rhs)).unwrap_err(),
            expected
        );
        assert_eq!(state.clock(), clock);
    }
    for (absolute, relative) in [
        ([0.0; 2], [0.0; 2]),
        ([f64::NAN; 2], [0.0; 2]),
        ([1e-10; 2], [-1.0; 2]),
        ([1e-10; 2], [f64::INFINITY; 2]),
    ] {
        let mut rhs = Source {
            bounds: Some(bounds()),
            value: 0.0,
            index: 0,
            refuse: false,
        };
        assert!(matches!(
            work.try_advance(
                &state,
                &mut candidate,
                8,
                Tolerances { absolute, relative },
                &mut rhs
            ),
            Err(SolverError::InvalidStep)
        ));
    }
}

#[test]
fn plan_source_spectrum_and_error_budget_refusals_are_structured() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let reservation = AttemptWorkspace::reservation(domain).unwrap();
    let approved = plan(Epoch(0), reservation);
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let state = SpectralState::from_rest(approved, clock, Epoch(0)).unwrap();
    let mut work = AttemptWorkspace::new(approved).unwrap();
    let policy = Tolerances {
        absolute: [1e-10; 2],
        relative: [1e-7; 2],
    };
    for case in 0..6 {
        let candidate_plan = if case == 0 {
            plan(Epoch(1), reservation)
        } else {
            approved
        };
        let mut candidate = CandidateState::new(candidate_plan, clock, Epoch(0)).unwrap();
        let mut rhs = Source {
            bounds: Some(bounds()),
            value: 100.0,
            index: 0,
            refuse: case == 1,
        };
        if case == 2 {
            rhs.index = 2;
        }
        let tolerances = if case == 3 {
            Tolerances {
                relative: [f64::MAX; 2],
                ..policy
            }
        } else {
            policy
        };
        let exhausted = SpectralState::from_rest(approved, clock, Epoch(u128::MAX)).unwrap();
        let incompatible =
            SpectralState::from_rest(plan(Epoch(1), reservation), clock, Epoch(0)).unwrap();
        let base = match case {
            4 => &exhausted,
            5 => &incompatible,
            _ => &state,
        };
        let expected = [
            SolverError::InvalidPayload,
            SolverError::ProviderBudgetExceeded,
            SolverError::InvalidSpectrum,
            SolverError::ArithmeticResolutionLimited,
            SolverError::EpochExhausted,
            SolverError::InvalidPayload,
        ][case];
        assert_eq!(
            (work.try_advance(base, &mut candidate, 8, tolerances, &mut rhs)).unwrap_err(),
            expected
        );
        assert_eq!(base.clock(), clock);
    }
}

#[test]
fn unrepresentable_modal_decay_is_an_arithmetic_failure_not_a_clock_repair() {
    for (viscosity, exponent) in [(f64::MAX, -6), (1.0, 1020)] {
        let domain = Domain::new([4; 3], [1.0; 3], viscosity).unwrap();
        let approved = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force: 128,
                diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
                overhead: 4096,
            },
            1024 * 1024,
            Epoch(0),
        )
        .unwrap();
        let clock = TickClock::from_rest(exponent, 64).unwrap();
        let (state, mut candidate, mut work) = transaction_support::setup(approved, clock);
        let mut rhs = Source {
            bounds: Some(bounds()),
            value: 0.0,
            index: 0,
            refuse: false,
        };
        let result = work.try_advance(
            &state,
            &mut candidate,
            8,
            Tolerances {
                absolute: [1e-10; 2],
                relative: [1e-7; 2],
            },
            &mut rhs,
        );
        assert_eq!(
            result.unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
        assert_eq!(state.clock(), clock);
    }
}
