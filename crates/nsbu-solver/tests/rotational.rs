//! Public rotational forcing and input-admission behavior.
use nsbu_solver::domain::Domain;
use nsbu_solver::spectral::RotationalWorkspace;
use nsbu_solver::{Complex64, SolverError};

#[test]
fn gradient_force_pressure_and_mean_acceleration_are_preserved() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let zero = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut force = [zero.clone(), zero.clone(), zero.clone()];
    let index = layout.locate([1, 1, 1]).unwrap().0;
    let expected = Complex64::new(0.25, -0.125);
    for (axis, values) in force.iter_mut().enumerate() {
        values[index] = Complex64::i() * std::f64::consts::TAU * expected;
        values[0] = Complex64::new((axis + 1) as f64, 0.0);
    }
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    let mut work = RotationalWorkspace::new(domain, 1024 * 1024).unwrap();
    let [a, b, c] = &mut output;
    work.evaluate(
        [&zero; 3],
        [&force[0], &force[1], &force[2]],
        [a, b, c],
        &mut pressure,
    )
    .unwrap();
    assert!((pressure[index] - expected).norm_sqr() < 1e-28);
    assert_eq!(pressure[0], Complex64::new(0.0, 0.0));
    for (axis, values) in output.iter().enumerate() {
        assert_eq!(values[0], Complex64::new((axis + 1) as f64, 0.0));
        assert!(values[index].norm_sqr() < 1e-28);
    }
}

#[test]
fn invalid_payload_and_cap_refusals_are_structured() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let bytes = RotationalWorkspace::reservation(domain).unwrap();
    assert!(matches!(
        RotationalWorkspace::new(domain, bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut work = RotationalWorkspace::new(domain, bytes).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); 48];
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    for (input_len, force_len, output_len, pressure_len) in [
        (47, 48, 48, 48),
        (48, 47, 48, 48),
        (48, 48, 47, 48),
        (48, 48, 48, 47),
    ] {
        let [a, b, c] = &mut output;
        assert_eq!(
            work.evaluate(
                [&zero[..input_len], &zero, &zero],
                [&zero[..force_len], &zero, &zero],
                [&mut a[..output_len], b, c],
                &mut pressure[..pressure_len]
            ),
            Err(SolverError::InvalidPayload)
        );
    }
}

#[test]
fn arithmetic_and_conjugacy_failures_preserve_input_state() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let zero = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut work = RotationalWorkspace::new(domain, 1024 * 1024).unwrap();
    for case in 0..4 {
        let mut velocity = [zero.clone(), zero.clone(), zero.clone()];
        velocity[0][0] = Complex64::new(1e200, 0.0);
        if case == 1 {
            velocity[1][layout.locate([1, 0, 0]).unwrap().0] = Complex64::new(1e200, 0.0);
            velocity[1][layout.locate([-1, 0, 0]).unwrap().0] = Complex64::new(1e200, 0.0);
        } else if case == 2 {
            velocity[0][0] = Complex64::new(1e8, 0.0);
            velocity[0][layout.locate([1, 0, 0]).unwrap().0] = Complex64::new(1e-8, 0.0);
        }
        if case == 3 {
            velocity[0][0] = Complex64::new(0.0, 0.0);
            for mode in [[1, 0, 0], [-1, 0, 0]] {
                velocity[1][layout.locate(mode).unwrap().0] = Complex64::new(2e307, 0.0);
            }
        }
        let before = velocity.clone();
        let mut output = [zero.clone(), zero.clone(), zero.clone()];
        let mut pressure = zero.clone();
        let [a, b, c] = &mut output;
        assert_eq!(
            work.evaluate(
                [&velocity[0], &velocity[1], &velocity[2]],
                [&zero; 3],
                [a, b, c],
                &mut pressure
            ),
            Err(SolverError::InvalidSpectrum),
            "case {case}"
        );
        assert_eq!(velocity, before);
    }
}

#[test]
fn force_conjugacy_is_checked_before_projection_with_relative_tolerance() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut work = RotationalWorkspace::new(domain, 1024 * 1024).unwrap();
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    for (mean, discrepancy, expected) in [
        (0.0, 1e-10, Err(SolverError::InvalidSpectrum)),
        (2.0, 64.0 * f64::EPSILON, Ok(())),
    ] {
        let mut force = zero.clone();
        force[0] = Complex64::new(mean, 0.0);
        force[domain.layout().locate([1, 0, 0]).unwrap().0] = Complex64::new(discrepancy, 0.0);
        let [a, b, c] = &mut output;
        assert_eq!(
            work.evaluate([&zero; 3], [&force, &zero, &zero], [a, b, c], &mut pressure),
            expected
        );
    }
}
