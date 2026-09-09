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
