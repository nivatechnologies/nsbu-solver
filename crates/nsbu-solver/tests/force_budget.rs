//! Provider admission and charged work on success, failure and exceeded declarations.
use nsbu_solver::domain::{Domain, TickClock};
use nsbu_solver::integrators::{
    forcing::{ForceLimits, ForceWork, PrescribedForce},
    kernel::RightHandSide,
    rhs::SpectralRhs,
};
use nsbu_solver::{Complex64, SolverError};

struct Source {
    limits: Option<ForceLimits>,
    work: ForceWork,
    fails: bool,
    changes: bool,
}
impl PrescribedForce for Source {
    fn limits(&self) -> Option<ForceLimits> {
        self.limits
    }
    fn evaluate(
        &mut self,
        _time: TickClock,
        _limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if self.fails {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if self.changes {
            self.limits = None;
        }
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
        }
        Ok(self.work)
    }
}
fn source() -> Source {
    Source {
        limits: Some(ForceLimits {
            storage_bytes: 128,
            work_units: 200,
            scalar_transforms: 1,
            remaining_divisor: 20,
        }),
        work: ForceWork {
            work_units: 100,
            scalar_transforms: 0,
        },
        fails: false,
        changes: false,
    }
}
fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}

#[test]
fn undeclared_and_overflowing_costs_are_refused_before_workspace_allocation() {
    let mut undeclared = source();
    undeclared.limits = None;
    assert!(matches!(
        SpectralRhs::new(domain(), undeclared, 0.3, 1024 * 1024),
        Err(SolverError::UnknownProviderCost)
    ));
    let limits = source().limits.unwrap();
    for invalid in [
        ForceLimits {
            work_units: 0,
            ..limits
        },
        ForceLimits {
            remaining_divisor: 0,
            ..limits
        },
    ] {
        assert_eq!(
            SpectralRhs::<Source>::reservation(domain(), invalid),
            Err(SolverError::UnknownProviderCost)
        );
    }
    for invalid in [
        ForceLimits {
            work_units: usize::MAX,
            ..limits
        },
        ForceLimits {
            scalar_transforms: usize::MAX,
            ..limits
        },
        ForceLimits {
            storage_bytes: usize::MAX,
            ..limits
        },
    ] {
        assert_eq!(
            SpectralRhs::<Source>::reservation(domain(), invalid),
            Err(SolverError::SizeOverflow)
        );
    }
    let bytes = SpectralRhs::<Source>::reservation(domain(), limits).unwrap();
    assert!(matches!(
        SpectralRhs::new(domain(), source(), 0.3, bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    for guard in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            SpectralRhs::new(domain(), source(), guard, bytes),
            Err(SolverError::InvalidStep)
        ));
    }
}

fn invoke(
    rhs: &mut SpectralRhs<Source>,
    input: &[Complex64],
    clock: TickClock,
) -> Result<(), SolverError> {
    let mut output = [input.to_vec(), input.to_vec(), input.to_vec()];
    let [a, b, c] = &mut output;
    rhs.evaluate([input; 3], clock, [a, b, c])
}

#[test]
fn exact_interval_call_cap_and_declared_work_are_enforced() {
    let clock = TickClock::from_rest(-10, 960).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); 48];
    let mut rhs = SpectralRhs::new(domain(), source(), 0.3, 1024 * 1024).unwrap();
    assert_eq!(
        invoke(&mut rhs, &zero, clock),
        Err(SolverError::ProviderBudgetExceeded)
    );
    assert_eq!(rhs.begin_attempt(clock, 52), Err(SolverError::InvalidStep));
    rhs.begin_attempt(clock, 48).unwrap();
    rhs.begin_attempt(clock, 4).unwrap();
    for _ in 0..12 {
        invoke(&mut rhs, &zero, clock).unwrap();
    }
    assert_eq!(rhs.consumption(), [12, 1200, 120]);
    assert_eq!(
        invoke(&mut rhs, &zero, clock),
        Err(SolverError::ProviderBudgetExceeded)
    );
}

#[test]
fn failed_or_dishonest_provider_reports_keep_the_complete_charged_budget() {
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); 48];
    for case in 0..4 {
        let mut provider = source();
        match case {
            0 => provider.fails = true,
            1 => provider.work.work_units = 201,
            2 => provider.work.scalar_transforms = 2,
            _ => provider.changes = true,
        }
        let mut rhs = SpectralRhs::new(domain(), provider, 0.3, 1024 * 1024).unwrap();
        rhs.begin_attempt(clock, 4).unwrap();
        let mut output = [zero.clone(), zero.clone(), zero.clone()];
        let [a, b, c] = &mut output;
        let result = rhs.evaluate([&zero; 3], clock, [a, b, c]);
        if case == 3 {
            assert_eq!(result, Ok(()));
            let [a, b, c] = &mut output;
            assert_eq!(
                rhs.evaluate([&zero; 3], clock, [a, b, c]),
                Err(SolverError::ProviderBudgetExceeded)
            );
        } else {
            let expected = if case == 0 {
                SolverError::ArithmeticResolutionLimited
            } else {
                SolverError::ProviderBudgetExceeded
            };
            assert_eq!(result, Err(expected));
            assert_eq!(rhs.consumption(), [1, 200, 11]);
        }
    }
}

#[test]
fn advective_refusal_preserves_the_supplied_state() {
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    let mut component = vec![Complex64::new(0.0, 0.0); 48];
    component[0] = Complex64::new(100.0, 0.0);
    let mut rhs = SpectralRhs::new(domain(), source(), 0.3, 1024 * 1024).unwrap();
    rhs.begin_attempt(clock, 4).unwrap();
    let mut output = [component.clone(), component.clone(), component.clone()];
    let [a, b, c] = &mut output;
    assert_eq!(
        rhs.evaluate([&component; 3], clock, [a, b, c]),
        Err(SolverError::AdvectiveLimit)
    );
    assert_eq!(component[0], Complex64::new(100.0, 0.0));
    assert_eq!(rhs.consumption(), [1, 100, 10]);
}

#[test]
fn declared_storage_transform_reports_and_exact_advective_boundary_are_preserved() {
    let domain = Domain::new([4; 3], [std::f64::consts::TAU; 3], 1.0).unwrap();
    let limits = source().limits.unwrap();
    let expected = nsbu_solver::spectral::RotationalWorkspace::reservation(domain).unwrap()
        + 48 * 4 * std::mem::size_of::<Complex64>()
        + limits.storage_bytes
        + std::mem::size_of::<SpectralRhs<Source>>();
    assert_eq!(
        SpectralRhs::<Source>::reservation(domain, limits),
        Ok(expected)
    );
    let mut provider = source();
    provider.work.scalar_transforms = 1;
    let mut rhs = SpectralRhs::new(domain, provider, 3.0 / 256.0, expected).unwrap();
    let bounds = rhs.bounds().unwrap();
    assert_eq!(
        (
            bounds.storage_bytes,
            bounds.work_units,
            bounds.scalar_transforms
        ),
        (expected, 200, 11)
    );
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    rhs.begin_attempt(clock, 4).unwrap();
    let mut mean = vec![Complex64::new(0.0, 0.0); 48];
    mean[0] = Complex64::new(1.0, 0.0);
    invoke(&mut rhs, &mean, clock).unwrap();
    assert_eq!(rhs.consumption(), [1, 100, 11]);
}

#[test]
fn selected_method_checks_its_complete_work_and_transform_counter_range() {
    use nsbu_solver::integrators::method::Method;
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    for (work, transforms, admitted) in [
        (200, usize::MAX / 100, true),
        (usize::MAX / 14, 1, false),
        (200, usize::MAX / 14, false),
        (usize::MAX / 15, 1, true),
        (200, usize::MAX / 15 - 10, true),
        (200, usize::MAX / 15 - 9, false),
    ] {
        let mut provider = source();
        let limits = provider.limits.as_mut().unwrap();
        limits.work_units = work;
        limits.scalar_transforms = transforms;
        let mut rhs = SpectralRhs::new(domain(), provider, 0.3, 1024 * 1024).unwrap();
        rhs.begin_attempt(clock, 4).unwrap();
        let result = rhs.begin_attempt_for_method(clock, 4, Method::HochbruckOstermann);
        if admitted {
            result.unwrap();
        } else {
            assert_eq!(result, Err(SolverError::SizeOverflow));
        }
        assert_eq!(rhs.consumption(), [0; 3]);
    }
}
