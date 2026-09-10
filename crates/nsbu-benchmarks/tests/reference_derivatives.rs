//! Full physical derivative tensors versus independent 120-digit Python fixtures.
use nsbu_benchmarks::{fields::reference, time::BenchmarkTime};
use nsbu_solver::domain::TickClock;

fn rational(text: &str) -> f64 {
    let mut parts = text.split('/');
    let numerator: f64 = parts.next().unwrap().parse().unwrap();
    numerator / parts.next().unwrap_or("1").parse::<f64>().unwrap()
}
fn time(text: &str) -> BenchmarkTime {
    let mut parts = text.split('/');
    let numerator: u128 = parts.next().unwrap().parse().unwrap();
    let denominator: u128 = parts.next().unwrap_or("1").parse().unwrap();
    // Preserve the same explicitly rounded 2^-100 late rational fixture clock as P05.
    let elapsed = (numerator * (1_u128 << 100) + denominator / 2) / denominator;
    let target = 1 << 93;
    BenchmarkTime::new(TickClock::restore(-100, target, elapsed, target - elapsed).unwrap())
        .unwrap()
}
fn flatten(sample: reference::ReferenceEvaluation) -> Vec<f64> {
    sample
        .velocity
        .into_iter()
        .chain(sample.gradient.into_iter().flatten())
        .chain(sample.hessian.into_iter().flatten().flatten())
        .chain(sample.vorticity)
        .chain([sample.pressure_raw])
        .chain(sample.pressure_gradient)
        .collect()
}

#[test]
fn every_tensor_entry_and_pressure_derivative_matches_independent_fixtures() {
    for line in include_str!("fixtures/derivatives.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let point = [
            rational(columns[1]),
            rational(columns[2]),
            rational(columns[3]),
        ];
        let sample = reference::evaluate(point, time(columns[4])).unwrap();
        let values = flatten(sample);
        assert_eq!(values.len(), 46);
        assert_eq!(columns.len(), 51);
        let mut maximum = 0.0_f64;
        for (index, (actual, text)) in values.iter().zip(&columns[5..]).enumerate() {
            let expected = rational(text);
            let scaled = (actual - expected).abs() / (1.0 + expected.abs());
            maximum = maximum.max(scaled);
            assert!(
                scaled < 2e-9,
                "{} entry {index}: {actual} vs {expected}",
                columns[0]
            );
        }
        for component in 0..3 {
            for a in 0..3 {
                for b in 0..3 {
                    assert_eq!(
                        sample.hessian[component][a][b],
                        sample.hessian[component][b][a]
                    );
                }
            }
        }
        println!(
            "reference derivatives {}: maximum_componentwise_scaled_error={maximum:e}",
            columns[0]
        );
    }
}

#[test]
fn exact_flat_states_periodicity_and_invalid_inputs_have_explicit_contracts() {
    for (point, clock) in [([0.125; 3], time("0")), ([0.5, 0.0, 0.0], time("1/256"))] {
        let flat = reference::evaluate(point, clock).unwrap();
        assert!(flat.root.is_none());
        assert!(flatten(flat).iter().all(|&value| value == 0.0));
    }
    let clock = time("1/256");
    let direct = reference::evaluate([0.125, -0.125, 0.0], clock).unwrap();
    assert!(direct.root.is_some());
    let periodic = reference::evaluate([1.125, 0.875, -1.0], clock).unwrap();
    assert_eq!(flatten(direct), flatten(periodic));
    assert!(reference::evaluate([f64::NAN, 0.0, 0.0], clock).is_err());
}
