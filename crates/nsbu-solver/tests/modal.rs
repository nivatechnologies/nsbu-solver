//! Modal physics includes pressure response to gradient forces and extreme scaling.
use nsbu_solver::domain::Domain;
use nsbu_solver::spectral::modal::{curl, modified_pressure, project, wavevector};
use nsbu_solver::{Complex64, SolverError};

#[test]
fn gradient_force_changes_pressure_without_accelerating_velocity() {
    let domain = Domain::new([8; 3], [1.0, 2.0, 4.0], 0.5).unwrap();
    let k = wavevector(domain, [-2, 1, 3]).unwrap();
    assert_eq!(
        k,
        [
            -2.0 * std::f64::consts::TAU,
            std::f64::consts::PI,
            1.5 * std::f64::consts::PI
        ]
    );
    let pressure = Complex64::new(2.0, -3.0);
    let force = k.map(|value| Complex64::i() * value * pressure);
    let projected = project(k, force).unwrap();
    assert!(projected.iter().all(|v| v.norm_sqr() < 1e-26));
    assert!((modified_pressure(k, force).unwrap() - pressure).norm_sqr() < 1e-28);
    assert!(curl(k, force).unwrap().iter().all(|v| v.norm_sqr() < 1e-24));
    let mean = [
        Complex64::new(1.0, 2.0),
        Complex64::new(3.0, 4.0),
        Complex64::new(5.0, 6.0),
    ];
    assert_eq!(project([0.0; 3], mean), Ok(mean));
    assert_eq!(
        modified_pressure([0.0; 3], mean),
        Ok(Complex64::new(0.0, 0.0))
    );
    assert_eq!(curl([0.0; 3], mean), Ok([Complex64::new(0.0, 0.0); 3]));
}

#[test]
fn projection_and_curl_follow_signed_cross_product_identities() {
    let k = [-2.0, 3.0, 4.0];
    let vector = [
        Complex64::new(1.0, -2.0),
        Complex64::new(3.0, 4.0),
        Complex64::new(-5.0, 6.0),
    ];
    let projected = project(k, vector).unwrap();
    let longitudinal = k
        .iter()
        .zip(projected)
        .map(|(a, b)| a * b)
        .sum::<Complex64>();
    assert!(longitudinal.norm_sqr() < 1e-28);
    let twice = project(k, projected).unwrap();
    assert!(projected
        .iter()
        .zip(twice)
        .all(|(a, b)| (*a - b).norm_sqr() < 1e-28));
    assert_eq!(
        curl(k, vector),
        Ok([
            Complex64::new(-2.0, -27.0),
            Complex64::new(-4.0, -6.0),
            Complex64::new(2.0, -9.0)
        ])
    );
    let large = project([1e300, 0.0, 0.0], vector).unwrap();
    assert_eq!(large, [Complex64::new(0.0, 0.0), vector[1], vector[2]]);
    assert_eq!(project([1e-300, 0.0, 0.0], vector).unwrap(), large);
}

#[test]
fn invalid_modes_nonfinite_data_and_arithmetic_overflow_are_refused() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    assert_eq!(
        wavevector(domain, [2, 0, 0]),
        Err(SolverError::InvalidIndex)
    );
    let tiny = Domain::new([4; 3], [f64::from_bits(1), 1.0, 1.0], 1.0).unwrap();
    assert_eq!(
        wavevector(tiny, [1, 0, 0]),
        Err(SolverError::InvalidSpectrum)
    );
    let zero = [Complex64::new(0.0, 0.0); 3];
    for k in [[f64::NAN, 0.0, 0.0], [0.0, f64::INFINITY, 0.0]] {
        assert_eq!(project(k, zero), Err(SolverError::InvalidSpectrum));
        assert_eq!(curl(k, zero), Err(SolverError::InvalidSpectrum));
        assert_eq!(
            modified_pressure(k, zero),
            Err(SolverError::InvalidSpectrum)
        );
    }
    for v in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        assert_eq!(project([1.0; 3], [v; 3]), Err(SolverError::InvalidSpectrum));
    }
    assert_eq!(
        curl([f64::MAX, 0.0, 0.0], [Complex64::new(2.0, 0.0); 3]),
        Err(SolverError::InvalidSpectrum)
    );
    assert_eq!(
        project([1.0; 3], [Complex64::new(f64::MAX, 0.0); 3]),
        Err(SolverError::InvalidSpectrum)
    );
    assert_eq!(
        modified_pressure([f64::from_bits(1), 0.0, 0.0], [Complex64::new(1.0, 0.0); 3]),
        Err(SolverError::InvalidSpectrum)
    );
}
