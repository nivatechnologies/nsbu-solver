//! Advective guards use the padded physical grid and refuse unrepresentable arithmetic.
use nsbu_solver::{domain::Domain, spectral::RotationalWorkspace, Complex64, SolverError};

#[test]
fn physical_grid_guard_matches_mean_velocity_and_checks_duration_arithmetic() {
    let domain = Domain::new([4, 8, 12], [1.0, 2.0, 4.0], 1.0).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut operator = RotationalWorkspace::new(domain, 1024 * 1024).unwrap();
    for scale in [1.0, 1e150] {
        let mut velocity = [zero.clone(), zero.clone(), zero.clone()];
        for (axis, component) in velocity.iter_mut().enumerate() {
            component[0] = Complex64::new((axis + 1) as f64 * scale, 0.0);
        }
        let mut output = [zero.clone(), zero.clone(), zero.clone()];
        let mut pressure = zero.clone();
        let [a, b, c] = &mut output;
        operator
            .evaluate(
                [&velocity[0], &velocity[1], &velocity[2]],
                [&zero; 3],
                [a, b, c],
                &mut pressure,
            )
            .unwrap();
        let expected = 0.125 * (1.0 + 3.0 + 3.75) * std::f64::consts::TAU * scale;
        assert!((operator.advective_number(0.125).unwrap() / expected - 1.0).abs() < 1e-14);
    }
    assert_eq!(
        operator.advective_number(1e200),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            operator.advective_number(duration),
            Err(SolverError::InvalidStep)
        );
    }
}
