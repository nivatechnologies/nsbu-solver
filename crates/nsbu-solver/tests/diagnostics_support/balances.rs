//! Independently integrated trigonometric energy/enstrophy fixtures.
use super::{
    diagnostics_support::{close, mode, slices, zero},
    evaluate,
};
use nsbu_solver::{
    diagnostics::{balances::measure, conservative::ConservativeWorkspace},
    domain::Domain,
    spectral::transfer,
    Complex64, SolverError,
};

#[test]
fn a_three_dimensional_triad_has_nonzero_stretching_and_known_balances() {
    // Exact symbolic integration over [-pi,pi]^3:
    // u=(sin(y), 2sin(z)+sin(x+z)/3, 3sin(x)).
    let source = Domain::new([4; 3], [std::f64::consts::TAU; 3], 0.25).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
    let mut velocity = zero(source.layout());
    for (axis, wave, amplitude) in [
        (0, [0, 1, 0], 1.0),
        (1, [0, 0, 1], 2.0),
        (2, [1, 0, 0], 3.0),
    ] {
        let mut value = [Complex64::new(0.0, 0.0); 3];
        value[axis] = Complex64::new(0.0, -amplitude / 2.0);
        mode(source.layout(), &mut velocity, wave, value);
    }
    let z = Complex64::new(0.0, 0.0);
    mode(
        source.layout(),
        &mut velocity,
        [1, 0, 1],
        [z, Complex64::new(0.0, -1.0 / 6.0), z],
    );
    let mut fine = zero(diagnostic.layout());
    for (input, output) in velocity.iter().zip(&mut fine) {
        transfer(source.layout(), diagnostic.layout(), input, output).unwrap();
    }
    let mut force = fine
        .clone()
        .map(|values| values.into_iter().map(|value| 2.0 * value).collect());
    // f=2u+grad(sin(x+z)); gradient work and curl vanish analytically.
    mode(
        diagnostic.layout(),
        &mut force,
        [1, 0, 1],
        [
            Complex64::new(0.5, 0.0),
            Complex64::new(0.0, -1.0 / 3.0),
            Complex64::new(0.5, 0.0),
        ],
    );
    let (conservative, _) = evaluate(source, &velocity, &force);
    let result = measure(
        diagnostic,
        slices(&fine),
        slices(&force),
        slices(&conservative),
    )
    .unwrap();
    close(result.energy, 127.0 / 36.0);
    close(result.enstrophy, 32.0 / 9.0);
    close(result.stretching, -0.5);
    close(result.energy_dissipation, 16.0 / 9.0);
    close(result.enstrophy_dissipation, 65.0 / 36.0);
    close(result.forcing_work, 127.0 / 9.0);
    close(result.vorticity_forcing, 128.0 / 9.0);
    close(result.norms.divergence_l2, 0.0);
}

#[test]
fn compensated_forcing_work_retains_a_small_term_between_large_cancellations() {
    for sign in [-1.0, 1.0] {
        let report = mean_work([2.0, 3.0, 4.0], [sign * 5e15, sign / 3.0, sign * -2.5e15]).unwrap();
        assert_eq!(report.forcing_work, sign);
    }
}

#[test]
fn large_mean_energy_is_computed_without_squaring_before_the_half_factor() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let mut velocity = zero(domain.layout());
    let force = zero(domain.layout());
    velocity[0][0] = Complex64::new(1.5e154, 0.0);
    let result = measure(domain, slices(&velocity), slices(&force), slices(&force)).unwrap();
    assert_eq!(result.energy, 0.75e154 * 1.5e154);
    velocity[0][0] = Complex64::new(2.0e154, 0.0);
    assert_eq!(
        measure(domain, slices(&velocity), slices(&force), slices(&force)).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}

#[test]
fn an_overflowing_forcing_contraction_is_not_reported_as_finite_work() {
    assert_eq!(
        mean_work([2.0, 0.0, 0.0], [f64::MAX, 0.0, 0.0]).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
}

fn mean_work(
    velocities: [f64; 3],
    forces: [f64; 3],
) -> Result<nsbu_solver::diagnostics::balances::BalanceSample, SolverError> {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let mut velocity = zero(domain.layout());
    let mut force = velocity.clone();
    let nonlinear = velocity.clone();
    for axis in 0..3 {
        velocity[axis][0] = Complex64::new(velocities[axis], 0.0);
        force[axis][0] = Complex64::new(forces[axis], 0.0);
    }
    measure(
        domain,
        slices(&velocity),
        slices(&force),
        slices(&nonlinear),
    )
}
