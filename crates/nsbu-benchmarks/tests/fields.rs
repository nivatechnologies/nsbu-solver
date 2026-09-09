//! Independent 120-digit pointwise fixture comparisons.
use nsbu_benchmarks::{fields, scalar, time::BenchmarkTime};
use nsbu_solver::domain::TickClock;

fn rational(text: &str) -> f64 {
    let mut parts = text.split('/');
    let numerator: f64 = parts.next().unwrap().parse().unwrap();
    numerator / parts.next().unwrap_or("1").parse::<f64>().unwrap()
}

#[test]
fn independent_high_precision_field_fixtures() {
    for line in include_str!("fixtures/fields.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let name = columns[0];
        let values: Vec<_> = columns[1..].iter().map(|s| rational(s)).collect();
        let point = [values[0], values[1], values[2]];
        let time = benchmark_time(columns[4]);
        let sample = fields::evaluate(point, time).unwrap();
        let independent = scalar::evaluate(point, time).unwrap();
        report(name, &sample, &values);
        for i in 0..3 {
            let expected = values[4 + i];
            assert!(
                (sample.velocity[i] - expected).abs() < 2e-10 * (1.0 + expected.abs()),
                "{name}: velocity {i}"
            );
            assert!(
                (independent.velocity[i] - expected).abs() < 2e-10 * (1.0 + expected.abs()),
                "{name}: scalar {i}"
            );
            let expected_force = values[7 + i];
            let actual_force = sample.force[i];
            assert!(
                (sample.force[i] - expected_force).abs() < 2e-9 * (1.0 + expected_force.abs()),
                "{name}: force {i}: {actual_force} vs {expected_force}"
            );
            for j in 0..3 {
                let expected_gradient = values[10 + 3 * i + j];
                let actual_gradient = sample.force_gradient[i][j];
                let cancellation_scale = sample.gradient_term_magnitudes[i][j];
                assert!(
                    (sample.force_gradient[i][j] - expected_gradient).abs()
                        < 2e-8 * (1.0 + expected_gradient.abs())
                            + 64.0 * f64::EPSILON * cancellation_scale,
                    "{name}: gradient {i},{j}: {actual_gradient} vs {expected_gradient}"
                );
            }
        }
        assert!(sample.divergence.abs() < 1e-9, "{name}: divergence");
        assert!(
            (sample.pressure_raw - independent.pressure_raw).abs()
                < 1e-9 * (1.0 + independent.pressure_raw.abs()),
            "{name}: pressure"
        );
    }
}

fn report(name: &str, sample: &nsbu_benchmarks::fields::Evaluation, values: &[f64]) {
    let gradient_absolute = sample
        .force_gradient
        .iter()
        .flatten()
        .zip(&values[10..])
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    let gradient_scaled = sample
        .force_gradient
        .iter()
        .flatten()
        .zip(&values[10..])
        .map(|(a, b)| (a - b).abs() / (1.0 + b.abs()))
        .fold(0.0_f64, f64::max);
    let force_scaled = sample
        .force
        .iter()
        .zip(&values[7..10])
        .map(|(a, b)| (a - b).abs() / (1.0 + b.abs()))
        .fold(0.0_f64, f64::max);
    let residual = sample
        .root_residual
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    let divergence = sample.divergence;
    println!("pointwise {name}: force_scaled={force_scaled:e} gradient_absolute={gradient_absolute:e} gradient_scaled={gradient_scaled:e} root_jet_residual_absolute={residual:e} divergence={divergence:e}");
}

fn benchmark_time(text: &str) -> BenchmarkTime {
    let mut fraction = text.split('/');
    let numerator: u128 = fraction.next().unwrap().parse().unwrap();
    let denominator: u128 = fraction.next().unwrap_or("1").parse().unwrap();
    // Nearest 2^-100 clock; the rational late sample is not dyadic.
    let ticks = (numerator * (1_u128 << 100) + denominator / 2) / denominator;
    let target = 1 << 93;
    BenchmarkTime::new(TickClock::restore(-100, target, ticks, target - ticks).unwrap()).unwrap()
}

#[test]
fn implicit_coefficients_match_independent_high_precision_jets() {
    for line in include_str!("fixtures/root-jets.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let name = columns[0];
        let z = nsbu_benchmarks::jet::Jet::variable(rational(columns[1]), 2).unwrap();
        let (q, _, _) = fields::implicit_root(z, benchmark_time(columns[2])).unwrap();
        for (coefficient, text) in q.coefficients().iter().zip(&columns[3..]) {
            let expected = rational(text);
            assert!(
                (coefficient - expected).abs() < 2e-13 * (1.0 + expected.abs()),
                "{name}: implicit coefficient"
            );
        }
    }
}
