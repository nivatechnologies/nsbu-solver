//! Diagnostic admission and floating arithmetic refusal controls.
use super::diagnostics_support::{slices, zero};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, conservative::ConservativeWorkspace},
    domain::Domain,
    Complex64, SolverError,
};

#[test]
fn comparison_requires_matching_geometry_and_valid_complete_spectra() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    for other in [
        Domain::new([4; 3], [2.0, 1.0, 1.0], 1.0).unwrap(),
        Domain::new([4; 3], [1.0; 3], 2.0).unwrap(),
        Domain::new([8; 3], [1.0; 3], 1.0).unwrap(),
    ] {
        assert_eq!(
            ComparisonPlan::new(other, domain).unwrap_err(),
            SolverError::InvalidDomain
        );
    }
    let plan = ComparisonPlan::new(domain, domain).unwrap();
    let valid = zero(domain.layout());
    for coefficient in [Complex64::new(f64::NAN, 0.0), Complex64::new(0.0, 1.0)] {
        let mut invalid = valid.clone();
        invalid[1][0] = coefficient;
        assert_eq!(
            plan.compare(slices(&invalid), slices(&valid)).unwrap_err(),
            SolverError::InvalidSpectrum
        );
        assert_eq!(
            plan.compare(slices(&valid), slices(&invalid)).unwrap_err(),
            SolverError::InvalidSpectrum
        );
    }
    let mut short = valid.clone();
    short[2].pop();
    assert_eq!(
        plan.compare(slices(&short), slices(&valid)).unwrap_err(),
        SolverError::InvalidPayload
    );
    assert_eq!(
        plan.compare(slices(&valid), slices(&short)).unwrap_err(),
        SolverError::InvalidPayload
    );
    let mut negative = valid.clone();
    let mut positive = valid;
    negative[0][0] = Complex64::new(-f64::MAX, 0.0);
    positive[0][0] = Complex64::new(f64::MAX, 0.0);
    assert_eq!(
        plan.compare(slices(&negative), slices(&positive))
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}

#[test]
fn conservative_preflight_and_output_shapes_are_enforced_before_evaluation() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let cap = ConservativeWorkspace::reservation(domain).unwrap();
    assert_eq!(
        ConservativeWorkspace::new(domain, cap - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let mut work = ConservativeWorkspace::new(domain, cap).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let velocity = zero(domain.layout());
    let force = zero(diagnostic.layout());
    for short_pressure in [false, true] {
        let mut output = force.clone();
        let mut pressure = force[0].clone();
        if short_pressure {
            pressure.pop();
        } else {
            output[0].pop();
        }
        let [a, b, c] = &mut output;
        assert_eq!(
            work.evaluate(slices(&velocity), slices(&force), [a, b, c], &mut pressure)
                .unwrap_err(),
            SolverError::InvalidPayload
        );
    }
    let unsupported = Domain::new([768; 3], [1.0; 3], 1.0).unwrap();
    assert_eq!(
        ConservativeWorkspace::reservation(unsupported).unwrap_err(),
        SolverError::InvalidDomain
    );
}
