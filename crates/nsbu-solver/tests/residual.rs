//! Physical derivative signs and newly resolved residual modes.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, norms::Norms, residual::ResidualPlan},
    domain::Domain,
    Complex64, SolverError,
};

#[test]
fn heat_flow_cancels_viscosity_while_unresolved_convection_remains_visible() {
    let domain = Domain::new([4; 3], [2.0, 3.0, 4.0], 0.25).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let mut velocity = zero(domain.layout());
    let mut derivative = zero(domain.layout());
    let mut convection = zero(diagnostic.layout());
    let z = Complex64::new(0.0, 0.0);
    let value = [Complex64::new(0.0, -0.5), z, z];
    let k = std::f64::consts::TAU / 4.0;
    mode(domain.layout(), &mut velocity, [0, 0, 1], value);
    mode(
        domain.layout(),
        &mut derivative,
        [0, 0, 1],
        value.map(|v| -0.25 * k * k * v),
    );
    mode(diagnostic.layout(), &mut convection, [0, 0, 2], value);
    let (norms, output) = measured(domain, &velocity, &derivative, &convection);
    close(norms.l2, 0.5_f64.sqrt());
    close(norms.h1, ((1.0 + 4.0 * k * k) / 2.0).sqrt());
    close(norms.vorticity_l2, 2.0 * k * 0.5_f64.sqrt());
    assert_eq!(norms.divergence_l2, 0.0);
    for (value, expected) in output.iter().flatten().zip(convection.iter().flatten()) {
        close(value.re, expected.re);
        close(value.im, expected.im);
    }
}

#[test]
fn constant_acceleration_is_not_removed_as_a_pressure_gauge() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let velocity = zero(domain.layout());
    let mut derivative = velocity.clone();
    let mut force_term = zero(diagnostic.layout());
    for (axis, value) in [1.0, -2.0, 3.0].into_iter().enumerate() {
        derivative[axis][0] = Complex64::new(value, 0.0);
        force_term[axis][0] = Complex64::new(-value, 0.0);
    }
    let (norms, _) = measured(domain, &velocity, &derivative, &force_term);
    assert_eq!(norms.l2, 0.0);
    force_term[0][0] = Complex64::new(0.0, 0.0);
    let (norms, output) = measured(domain, &velocity, &derivative, &force_term);
    assert_eq!(norms.l2, 1.0);
    assert_eq!(norms.h1, 1.0);
    assert_eq!(output[0][0], Complex64::new(1.0, 0.0));
}

fn measured(
    domain: Domain,
    velocity: &[Vec<Complex64>; 3],
    derivative: &[Vec<Complex64>; 3],
    nonlinear: &[Vec<Complex64>; 3],
) -> (Norms, [Vec<Complex64>; 3]) {
    let mut output = nonlinear.clone();
    let [a, b, c] = &mut output;
    let norms = ResidualPlan::new(domain)
        .unwrap()
        .evaluate(
            slices(velocity),
            slices(derivative),
            slices(nonlinear),
            [a, b, c],
        )
        .unwrap();
    (norms, output)
}

#[test]
fn residual_refuses_incomplete_output() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let input = zero(domain.layout());
    let nonlinear = zero(diagnostic.layout());
    let mut output = nonlinear.clone();
    output[1].pop();
    let [a, b, c] = &mut output;
    assert_eq!(
        ResidualPlan::new(domain)
            .unwrap()
            .evaluate(
                slices(&input),
                slices(&input),
                slices(&nonlinear),
                [a, b, c]
            )
            .unwrap_err(),
        SolverError::InvalidPayload
    );
}
