use super::support::direct;
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::FftPlan;
use nsbu_solver::{Complex64, SolverError};

#[test]
fn specialized_butterflies_match_zero_heavy_nyquist_oracles() {
    for dimensions in [[6, 6, 6], [18, 2, 2], [144, 2, 2]] {
        let layout = Layout::new(dimensions).unwrap();
        let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
        let mut values = vec![-0.0; layout.real_len()];
        values[0] = 1.0;
        values[(dimensions[0] / 2 * dimensions[1]) * dimensions[2]] = -0.5;
        values[dimensions[2] / 2] = 0.25;
        let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        plan.forward(&values, &mut coefficients, &mut work).unwrap();
        for mode in [
            [0, 0, 0],
            [dimensions[0] / 2, 0, 0],
            [0, dimensions[1] / 2, dimensions[2] / 2],
            [dimensions[0] - 1, dimensions[1] - 1, dimensions[2] / 2],
        ] {
            let actual = coefficients[layout.index(mode).unwrap()];
            assert!((actual - direct(&values, dimensions, mode)).norm_sqr() < 5e-26);
        }
        let mut restored = vec![0.0; layout.real_len()];
        plan.inverse(&coefficients, &mut restored, &mut work)
            .unwrap();
        assert!(values
            .iter()
            .zip(restored)
            .all(|(a, b)| (a - b).abs() < 3e-13));
    }
}

#[test]
fn specialized_butterflies_handle_large_finite_and_overflow_inputs() {
    for dimensions in [[6, 2, 2], [18, 2, 2], [144, 2, 2]] {
        let layout = Layout::new(dimensions).unwrap();
        let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
        let amplitude = f64::MAX / 8.0;
        let mut values = vec![0.0; layout.real_len()];
        values[0] = amplitude;
        let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        plan.forward(&values, &mut coefficients, &mut work).unwrap();
        assert!(coefficients
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite()));
        let mut restored = vec![0.0; layout.real_len()];
        plan.inverse(&coefficients, &mut restored, &mut work)
            .unwrap();
        assert!((restored[0] / amplitude - 1.0).abs() < 3e-13);
        assert!(restored[1..]
            .iter()
            .all(|value| value.abs() / amplitude < 3e-13));
        values.fill(f64::MAX);
        assert_eq!(
            plan.forward(&values, &mut coefficients, &mut work),
            Err(SolverError::InvalidSpectrum)
        );
    }
}
