//! Independent hypergeometric fixtures and nonzero/zero operator tableau identities.
use nsbu_solver::{integrators::ho_coefficients::HoCoefficients, SolverError};

#[test]
fn independent_high_precision_coefficients_cover_junctions_and_extremes() {
    let mut maxima = [0.0_f64; 2];
    for line in include_str!("fixtures/ho-coefficients.tsv").lines() {
        let values: Vec<f64> = line
            .split('\t')
            .map(|value| value.parse().unwrap())
            .collect();
        let z = values[0];
        let table = HoCoefficients::new(z).unwrap();
        let actual = [table.exponential, table.half_exponential]
            .into_iter()
            .chain(table.rows.into_iter().flatten())
            .chain(table.weights);
        for (value, expected) in actual.zip(&values[1..]) {
            let scale = expected.abs().max(1.0 / (1.0 + z.abs()));
            let difference = (value - expected).abs();
            maxima[0] = maxima[0].max(difference);
            maxima[1] = maxima[1].max(difference / scale);
            let tolerance = 64.0 * f64::EPSILON * scale + 1e-323;
            assert!(
                (value - expected).abs() <= tolerance,
                "z={z} value={value:e} expected={expected:e}"
            );
        }
    }
    println!(
        "HO coefficients max_absolute={:e} max_conditioning_scaled={:e}",
        maxima[0], maxima[1]
    );
}

#[test]
fn arguments_and_analytic_truncation_are_explicit() {
    for z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e-300] {
        assert_eq!(
            HoCoefficients::new(z).unwrap_err(),
            SolverError::InvalidStep
        );
    }
    for z in [-2.0, -1.0, 0.0] {
        assert_eq!(HoCoefficients::new(z).unwrap().truncation_bound, 1e-23);
    }
    assert_eq!(HoCoefficients::new(-2.0001).unwrap().truncation_bound, 0.0);
}

fn dot(a: [f64; 5], b: [f64; 5]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}

#[test]
fn zero_operator_satisfies_all_eight_fourth_order_conditions() {
    let table = HoCoefficients::new(0.0).unwrap();
    let c: [f64; 5] = [0.0, 0.5, 0.5, 1.0, 0.5];
    let square = c.map(|v| v * v);
    let ac = table.rows.map(|row| dot(row, c));
    let conditions = [
        dot(table.weights, [1.0; 5]),
        dot(table.weights, c),
        dot(table.weights, square),
        dot(table.weights, c.map(|v| v * v * v)),
        dot(table.weights, ac),
        dot(table.weights, table.rows.map(|row| dot(row, square))),
        dot(table.weights, std::array::from_fn(|i| c[i] * ac[i])),
        dot(table.weights, table.rows.map(|row| dot(row, ac))),
    ];
    for (actual, expected) in conditions.into_iter().zip([
        1.0,
        0.5,
        1.0 / 3.0,
        0.25,
        1.0 / 6.0,
        1.0 / 12.0,
        0.125,
        1.0 / 24.0,
    ]) {
        assert!((actual - expected).abs() < 4e-16);
    }
}

#[test]
fn dissipative_row_and_weight_identities_hold_away_from_zero() {
    let nodes = [0.0, 0.5, 0.5, 1.0, 0.5];
    for z in [-0.25, -1.0, -1.5, -2.0, -10.0, -1000.0] {
        let table = HoCoefficients::new(z).unwrap();
        for (row, node) in table.rows.into_iter().zip(nodes).skip(1) {
            let expected = (node * z).exp_m1() / z;
            assert!((row.iter().sum::<f64>() - expected).abs() < 8e-16);
        }
        for i in [3, 4] {
            let argument = nodes[i] * z;
            let expected = ((argument.exp_m1() / argument) - 1.0) / (z * z) * argument;
            assert!((dot(table.rows[i], nodes) - expected).abs() < 8e-16);
        }
        assert!((table.weights.iter().sum::<f64>() - z.exp_m1() / z).abs() < 8e-16);
    }
}
