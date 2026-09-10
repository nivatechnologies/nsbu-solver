//! Independent continuum forcing coefficients and smooth-provider refusals.
mod fixture_support;
use nsbu_benchmarks::smooth::{amplitude, CyclicSine};
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};

#[test]
fn analytical_coefficients_agree_with_high_precision_physical_dft() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let mut source = CyclicSine::new(domain).unwrap();
    let limits = source.limits().unwrap();
    assert_eq!(limits.storage_bytes, std::mem::size_of::<CyclicSine>());
    assert_eq!(limits.work_units, 3 * domain.layout().half_len());
    assert_eq!(limits.remaining_divisor, 1);
    assert_eq!(limits.scalar_transforms, 0);
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let [a, b, c] = &mut output;
    let used = source
        .evaluate(
            TickClock::restore(-10, 1024, 16, 1008).unwrap(),
            limits,
            [a, b, c],
        )
        .unwrap();
    assert_eq!(used.work_units, limits.work_units);
    assert_eq!(used.scalar_transforms, 0);
    fixture_support::compare(
        domain.layout(),
        &output,
        include_str!("fixtures/smooth-force.tsv"),
        1e-13,
    );
    for values in output {
        nsbu_solver::domain::validate_spectrum(domain.layout(), &values, 0.0).unwrap();
    }
}

#[test]
fn source_refuses_invalid_geometry_budget_shape_and_arithmetic() {
    assert_eq!(
        CyclicSine::new(Domain::new([4; 3], [2.0; 3], 1.0).unwrap()).unwrap_err(),
        SolverError::InvalidDomain
    );
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let mut source = CyclicSine::new(domain).unwrap();
    let limits = source.limits().unwrap();
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    assert_eq!(
        amplitude(clock).unwrap(),
        [(0.0, 13.0), (0.0, 17.0), (0.0, 19.0)]
    );
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let [a, b, c] = &mut output;
    let mut wrong = limits;
    wrong.work_units = 0;
    assert_eq!(
        source.evaluate(clock, wrong, [a, b, c]).unwrap_err(),
        SolverError::ProviderBudgetExceeded
    );
    assert_eq!(
        source
            .evaluate(clock, limits, [&mut a[..1], b, c])
            .unwrap_err(),
        SolverError::InvalidPayload
    );
    let huge = TickClock::restore(1023, 2, 1, 1).unwrap();
    assert_eq!(
        source.evaluate(huge, limits, [a, b, c]).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    let too_fine = TickClock::restore(-1075, 2, 1, 1).unwrap();
    assert_eq!(
        amplitude(too_fine).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    let mut stiff = CyclicSine::new(Domain::new([4; 3], [1.0; 3], f64::MAX).unwrap()).unwrap();
    assert_eq!(
        stiff
            .evaluate(clock, stiff.limits().unwrap(), [a, b, c])
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}
