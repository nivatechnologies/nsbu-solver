use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftPlan, FftWorkspace};
use nsbu_solver::{Complex64, SolverError};

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
