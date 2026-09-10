//! Analytic full-band conservative-product and physical-pressure controls.
#[path = "diagnostics_support/balances.rs"]
mod balances;
mod diagnostics_support;
#[path = "diagnostics_support/refusals.rs"]
mod refusals;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{diagnostics::conservative::ConservativeWorkspace, domain::Domain, Complex64};

fn evaluate(
    domain: Domain,
    velocity: &[Vec<Complex64>; 3],
    force: &[Vec<Complex64>; 3],
) -> ([Vec<Complex64>; 3], Vec<Complex64>) {
    let cap = ConservativeWorkspace::reservation(domain).unwrap();
    let mut work = ConservativeWorkspace::new(domain, cap).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let mut result = zero(diagnostic.layout());
    let mut pressure = vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()];
    let [a, b, c] = &mut result;
    work.evaluate(slices(velocity), slices(force), [a, b, c], &mut pressure)
        .unwrap();
    (result, pressure)
}

#[test]
fn gradient_force_changes_pressure_without_accelerating_velocity() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let velocity = zero(domain.layout());
    let mut force = zero(diagnostic.layout());
    let z = Complex64::new(0.0, 0.0);
    for wave in [[1, 0, 0], [1, 1, 1]] {
        mode(
            diagnostic.layout(),
            &mut force,
            wave,
            wave.map(|k| Complex64::new(std::f64::consts::PI * k as f64, 0.0)),
        );
    }
    force[1][0] = Complex64::new(2.0, 0.0);
    let (result, pressure) = evaluate(domain, &velocity, &force);
    let mut expected = zero(diagnostic.layout());
    expected[1][0] = Complex64::new(-2.0, 0.0);
    assert_eq!(result, expected);
    for wave in [[1, 0, 0], [1, 1, 1]] {
        mode(
            diagnostic.layout(),
            &mut expected,
            wave,
            [Complex64::new(0.0, -0.5), z, z],
        );
    }
    assert_eq!(pressure, expected[0]);
}

#[test]
fn taylor_green_pressure_contains_modes_outside_the_velocity_band() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let mut velocity = zero(domain.layout());
    let z = Complex64::new(0.0, 0.0);
    for sign in [-1, 1] {
        mode(
            domain.layout(),
            &mut velocity,
            [1, sign, 0],
            [
                Complex64::new(0.0, -0.25),
                Complex64::new(0.0, 0.25 * sign as f64),
                z,
            ],
        );
    }
    let (result, pressure) = evaluate(domain, &velocity, &zero(diagnostic.layout()));
    for value in result.iter().flatten() {
        close(value.re.hypot(value.im), 0.0);
    }
    let mut expected = zero(diagnostic.layout());
    for wave in [[2, 0, 0], [0, 2, 0]] {
        mode(
            diagnostic.layout(),
            &mut expected,
            wave,
            [Complex64::new(0.125, 0.0), z, z],
        );
    }
    for (value, expected) in pressure.into_iter().zip(&expected[0]) {
        close(value.re, expected.re);
        close(value.im, expected.im);
    }
}

#[test]
fn cyclic_sines_have_transverse_conservative_acceleration() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let mut velocity = zero(domain.layout());
    let mut expected = zero(diagnostic.layout());
    for axis in 0..3 {
        let mut wave = [0; 3];
        wave[(axis + 1) % 3] = 1;
        let mut value = [Complex64::new(0.0, 0.0); 3];
        value[axis] = Complex64::new(0.0, -0.5);
        mode(domain.layout(), &mut velocity, wave, value);
        // cos(next coordinate) * sin(following coordinate), with physical derivative k.
        for sign in [-1, 1] {
            wave[(axis + 1) % 3] = sign;
            wave[(axis + 2) % 3] = 1;
            value[axis] = Complex64::new(0.0, -std::f64::consts::TAU / 4.0);
            mode(diagnostic.layout(), &mut expected, wave, value);
        }
    }
    let (result, pressure) = evaluate(domain, &velocity, &zero(diagnostic.layout()));
    for (value, expected) in result.iter().flatten().zip(expected.iter().flatten()) {
        close(value.re, expected.re);
        close(value.im, expected.im);
    }
    for value in pressure {
        close(value.re.hypot(value.im), 0.0);
    }
}
