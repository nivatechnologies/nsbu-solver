use super::support::{direct, direct_inverse, hermitian_fixture, same_bits};
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::FftPlan;
use nsbu_solver::Complex64;

#[test]
fn mixed_radix_anisotropic_forward_matches_independent_direct_sums() {
    for dimensions in [[4, 4, 4], [4, 6, 8], [6, 12, 18], [8, 8, 8], [12, 12, 12]] {
        let layout = Layout::new(dimensions).unwrap();
        let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
        let values = (0..layout.real_len())
            .map(|i| ((17 * i + 3) % 101) as f64 / 101.0 - 0.25)
            .collect::<Vec<_>>();
        let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        plan.forward(&values, &mut coefficients, &mut work).unwrap();
        for mode in [
            [0, 0, 0],
            [1, 2, 1],
            [dimensions[0] - 1, dimensions[1] - 1, dimensions[2] / 2],
            [2, 1, 0],
        ] {
            let expected = direct(&values, dimensions, mode);
            let actual = coefficients[layout.index(mode).unwrap()];
            assert!(
                (actual - expected).norm_sqr() < 1e-27,
                "{dimensions:?} {mode:?}: {actual} vs {expected}"
            );
        }
        let mut restored = vec![0.0; values.len()];
        plan.inverse(&coefficients, &mut restored, &mut work)
            .unwrap();
        assert!(values
            .iter()
            .zip(restored)
            .all(|(a, b)| (a - b).abs() < 3e-14));
    }
}

#[test]
fn constant_and_negative_frequency_modes_keep_normalized_amplitudes() {
    let layout = Layout::new([8; 3]).unwrap();
    let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
    let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    coefficients[0] = Complex64::new(2.0, 0.0);
    coefficients[layout.index([7, 1, 2]).unwrap()] = Complex64::new(0.5, -0.25);
    let mut values = vec![0.0; layout.real_len()];
    plan.inverse(&coefficients, &mut values, &mut work).unwrap();
    for i in 0..8 {
        for j in 0..8 {
            for k in 0..8 {
                let phase = std::f64::consts::TAU * (-(i as f64) + j as f64 + 2.0 * k as f64) / 8.0;
                let expected = 2.0 + phase.cos() + 0.5 * phase.sin();
                assert!((values[(i * 8 + j) * 8 + k] - expected).abs() < 3e-14);
            }
        }
    }
    let mut roundtrip = coefficients.clone();
    plan.forward(&values, &mut roundtrip, &mut work).unwrap();
    assert!(coefficients
        .iter()
        .zip(roundtrip)
        .all(|(a, b)| (*a - b).norm_sqr() < 1e-27));
}

#[test]
fn full_anisotropic_oracle_covers_rows_hermitian_planes_and_nyquist() {
    let layout = Layout::new([4, 6, 8]).unwrap();
    let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
    let coefficients = hermitian_fixture(layout);
    let mut physical = vec![0.0; layout.real_len()];
    plan.inverse(&coefficients, &mut physical, &mut work)
        .unwrap();
    for i in 0..4 {
        for j in 0..6 {
            for k in 0..8 {
                let expected = direct_inverse(&coefficients, layout, [i, j, k]);
                let actual = physical[(i * 6 + j) * 8 + k];
                assert!((actual - expected).abs() < 5e-13, "{i}, {j}, {k}");
            }
        }
    }
    let mut restored = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    plan.forward(&physical, &mut restored, &mut work).unwrap();
    assert!(coefficients
        .iter()
        .zip(restored)
        .all(|(a, b)| (*a - b).norm_sqr() < 3e-26));
}

#[test]
fn repeated_transform_output_is_bitwise_deterministic() {
    let layout = Layout::new([4, 6, 8]).unwrap();
    let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
    let values = (0..layout.real_len())
        .map(|index| ((29 * index + 17) % 113) as f64 / 113.0 - 0.5)
        .collect::<Vec<_>>();
    let mut coefficients = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut physical = vec![0.0; layout.real_len()];
    plan.forward(&values, &mut coefficients, &mut work).unwrap();
    plan.inverse(&coefficients, &mut physical, &mut work)
        .unwrap();
    let expected_coefficients = coefficients.clone();
    let expected_physical = physical.clone();
    for _ in 0..16 {
        plan.forward(&values, &mut coefficients, &mut work).unwrap();
        plan.inverse(&coefficients, &mut physical, &mut work)
            .unwrap();
        assert!(same_bits(&coefficients, &expected_coefficients));
        assert!(physical
            .iter()
            .zip(&expected_physical)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
    }
}
