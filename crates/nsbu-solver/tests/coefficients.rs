//! Independent 80/120-digit hypergeometric fixtures and modal integration identities.
use nsbu_solver::integrators::coefficients::CmCoefficients;
use nsbu_solver::SolverError;

#[test]
fn all_coefficients_match_high_precision_at_junctions_and_weight_zeros() {
    for line in include_str!("fixtures/cm-coefficients.tsv").lines() {
        let values: Vec<f64> = line
            .split('\t')
            .map(|value| value.parse().unwrap())
            .collect();
        assert_eq!(values.len(), 7);
        let z = values[0];
        let c = CmCoefficients::new(z).unwrap();
        let actual = [
            c.exponential,
            c.half_exponential,
            c.q,
            c.weights[0],
            c.weights[1],
            c.weights[2],
        ];
        for (a, expected) in actual.into_iter().zip(&values[1..]) {
            assert!(a.is_finite());
            assert!(
                (a - expected).abs() <= 4e-15,
                "z={z}, actual={a}, expected={expected}"
            );
        }
        assert!(c.truncation_bound >= 0.0);
        assert!(c.truncation_bound <= 1e-17);
    }
}

#[test]
fn zero_operator_and_constant_source_identities_hold() {
    let c = CmCoefficients::new(0.0).unwrap();
    assert_eq!((c.exponential, c.half_exponential, c.q), (1.0, 1.0, 0.5));
    for (actual, expected) in c.weights.into_iter().zip([1.0 / 6.0, 1.0 / 3.0, 1.0 / 6.0]) {
        assert!((actual - expected).abs() < 2e-16);
    }
    for z in [-0.01, -1.0, -2.0, -49.0, -50.0, -100.0, -1e200] {
        let c = CmCoefficients::new(z).unwrap();
        let expected = z.exp_m1() / z;
        assert!((c.weights[0] + 2.0 * c.weights[1] + c.weights[2] - expected).abs() < 4e-15);
    }
    for z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e-300] {
        assert_eq!(CmCoefficients::new(z), Err(SolverError::InvalidStep));
    }
}
