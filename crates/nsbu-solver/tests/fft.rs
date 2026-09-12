//! Independent direct sums test actual Fourier amplitudes, not only round trips.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::FftPlan;
use nsbu_solver::{Complex64, SolverError};

fn direct(values: &[f64], dimensions: [usize; 3], mode: [usize; 3]) -> Complex64 {
    let [nx, ny, nz] = dimensions;
    let mut sum = Complex64::new(0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let phase = -std::f64::consts::TAU
                    * (i as f64 * mode[0] as f64 / nx as f64
                        + j as f64 * mode[1] as f64 / ny as f64
                        + k as f64 * mode[2] as f64 / nz as f64);
                sum += values[(i * ny + j) * nz + k] * Complex64::new(phase.cos(), phase.sin());
            }
        }
    }
    sum / values.len() as f64
}

fn direct_inverse(coefficients: &[Complex64], layout: Layout, point: [usize; 3]) -> f64 {
    let [nx, ny, nz] = layout.dimensions();
    let mut sum = Complex64::new(0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..=nz / 2 {
                let phase = std::f64::consts::TAU
                    * (i as f64 * point[0] as f64 / nx as f64
                        + j as f64 * point[1] as f64 / ny as f64
                        + k as f64 * point[2] as f64 / nz as f64);
                let exponential = Complex64::new(phase.cos(), phase.sin());
                let value = coefficients[layout.index([i, j, k]).unwrap()];
                sum += value * exponential;
                if k != 0 && k != nz / 2 {
                    sum += value.conj() * exponential.conj();
                }
            }
        }
    }
    sum.re
}

fn hermitian_fixture(layout: Layout) -> Vec<Complex64> {
    let [nx, ny, nz] = layout.dimensions();
    let mut values = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for k in 1..nz / 2 {
        for i in 0..nx {
            for j in 0..ny {
                values[layout.index([i, j, k]).unwrap()] = Complex64::new(
                    (11 * i + 7 * j + 5 * k) as f64 / 97.0,
                    (3 * i + 13 * j + 2 * k) as f64 / 89.0 - 0.5,
                );
            }
        }
    }
    for k in [0, nz / 2] {
        for i in 0..nx {
            for j in 0..ny {
                let partner = [(nx - i) % nx, (ny - j) % ny, k];
                let position = [i, j, k];
                if layout.index(position).unwrap() > layout.index(partner).unwrap() {
                    continue;
                }
                let value = Complex64::new(
                    (17 * i + 5 * j + 3 * k) as f64 / 101.0,
                    (7 * i + 11 * j + k) as f64 / 103.0 - 0.25,
                );
                let owned = if position == partner {
                    Complex64::new(value.re, 0.0)
                } else {
                    value
                };
                values[layout.index(position).unwrap()] = owned;
                values[layout.index(partner).unwrap()] = owned.conj();
            }
        }
    }
    values
}

fn same_bits(left: &[Complex64], right: &[Complex64]) -> bool {
    left.iter()
        .zip(right)
        .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}

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

#[test]
fn plans_refuse_unsupported_lengths_memory_caps_and_payloads() {
    for dims in [[10, 4, 4], [4, 14, 4], [4, 4, 2048]] {
        assert_eq!(
            FftPlan::reservation(Layout::new(dims).unwrap()),
            Err(SolverError::InvalidDomain)
        );
    }
    let layout = Layout::new([4; 3]).unwrap();
    let bytes = FftPlan::reservation(layout).unwrap();
    assert!(matches!(
        FftPlan::new(layout, bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let (plan, mut work) = FftPlan::new(layout, bytes).unwrap();
    let (_, mut other) = FftPlan::new(Layout::new([8; 3]).unwrap(), 1024 * 1024).unwrap();
    let mut real = vec![0.0; 64];
    let mut complex = vec![Complex64::new(0.0, 0.0); 48];
    assert_eq!(
        plan.forward(&real, &mut complex, &mut other),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        plan.forward(&real[..63], &mut complex, &mut work),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        plan.forward(&real, &mut complex[..47], &mut work),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        plan.inverse(&complex[..47], &mut real, &mut work),
        Err(SolverError::InvalidPayload)
    );
    real[0] = f64::NAN;
    assert_eq!(
        plan.forward(&real, &mut complex, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
    complex[0].re = f64::INFINITY;
    assert_eq!(
        plan.inverse(&complex, &mut real, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
    complex[0] = Complex64::new(0.0, f64::NAN);
    assert_eq!(
        plan.inverse(&complex, &mut real, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
}

#[test]
fn inverse_checks_both_conjugate_planes_and_declared_roundoff_boundary() {
    let layout = Layout::new([4; 3]).unwrap();
    let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
    let mut real = vec![0.0; 64];
    for (position, value) in [
        ([1, 0, 0], Complex64::new(1.0, 0.0)),
        ([1, 0, 2], Complex64::new(0.0, 1.0)),
        ([0, 0, 0], Complex64::new(0.0, 0.25)),
    ] {
        let mut spectrum = vec![Complex64::new(0.0, 0.0); 48];
        spectrum[layout.index(position).unwrap()] = value;
        assert_eq!(
            plan.inverse(&spectrum, &mut real, &mut work),
            Err(SolverError::InvalidSpectrum)
        );
    }
    let mut spectrum = vec![Complex64::new(0.0, 0.0); 48];
    spectrum[0] = Complex64::new(0.0, 32.0 * f64::EPSILON);
    plan.inverse(&spectrum, &mut real, &mut work).unwrap();
    spectrum[0].im = 33.0 * f64::EPSILON;
    assert_eq!(
        plan.inverse(&spectrum, &mut real, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
}

#[test]
fn finite_input_that_overflows_transform_arithmetic_is_refused() {
    let layout = Layout::new([4; 3]).unwrap();
    let (plan, mut work) = FftPlan::new(layout, 1024 * 1024).unwrap();
    let mut real = vec![f64::MAX; 64];
    let mut spectrum = vec![Complex64::new(0.0, 0.0); 48];
    assert_eq!(
        plan.forward(&real, &mut spectrum, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
    spectrum.fill(Complex64::new(f64::MAX, 0.0));
    assert_eq!(
        plan.inverse(&spectrum, &mut real, &mut work),
        Err(SolverError::InvalidSpectrum)
    );
}

#[test]
fn reservation_accounts_for_every_owned_buffer_and_inclusive_axis_limit() {
    use nsbu_solver::spectral::FftWorkspace;
    for dimensions in [[4, 6, 8], [1024, 2, 2]] {
        let layout = Layout::new(dimensions).unwrap();
        let expected = 16
            * (layout.half_len()
                + 3 * dimensions.iter().max().unwrap()
                + dimensions.iter().sum::<usize>())
            + std::mem::size_of::<FftPlan>()
            + std::mem::size_of::<FftWorkspace>();
        assert_eq!(FftPlan::reservation(layout), Ok(expected));
    }
    assert_eq!(
        FftPlan::reservation(Layout::new([1536, 2, 2]).unwrap()),
        Err(SolverError::InvalidDomain)
    );
}
