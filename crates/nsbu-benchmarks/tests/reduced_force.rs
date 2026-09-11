//! Independent high-precision force fixture checks for the reduced variable arithmetic.
use nsbu_benchmarks::{fields, reduced_force, time::BenchmarkTime};
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
    let elapsed = (numerator * (1u128 << 100) + denominator / 2) / denominator;
    let target = 1u128 << 93;
    BenchmarkTime::new(TickClock::restore(-100, target, elapsed, target - elapsed).unwrap())
        .unwrap()
}
#[test]
fn high_precision_values_and_complete_cartesian_momentum_terms_agree() {
    assert_eq!(include_str!("fixtures/fields.tsv").lines().count(), 9);
    for line in include_str!("fixtures/fields.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let point = std::array::from_fn(|i| rational(columns[1 + i]));
        let clock = time(columns[4]);
        let actual = reduced_force::evaluate(point, clock).unwrap();
        let full = fields::evaluate(point, clock).unwrap();
        let mut maximum = 0.0_f64;
        for i in 0..3 {
            let expected = rational(columns[8 + i]);
            let error = (actual.force[i] - expected).abs() / (1.0 + expected.abs());
            maximum = maximum.max(error);
            assert!(
                error < 5e-12,
                "{} force {i}: {} vs {expected}, scaled {error}",
                columns[0],
                actual.force[i]
            );
            for j in 0..4 {
                let a = actual.momentum_terms[i][j];
                let b = full.momentum_terms[i][j];
                assert!(
                    (a - b).abs() < 5e-12 * (1.0 + b.abs()),
                    "{} term {i},{j}: {a} vs {b}",
                    columns[0]
                );
            }
            let expected = rational(columns[5 + i]);
            assert!((actual.velocity[i] - expected).abs() < 2e-10 * (1.0 + expected.abs()));
        }
        assert!(
            (actual.pressure_raw - full.pressure_raw).abs()
                < 2e-10 * (1.0 + full.pressure_raw.abs())
        );
        println!("reduced force {} scaled_error={maximum:e}", columns[0]);
    }
}

#[test]
fn exact_word_inputs_match_80_120_digit_refinements_through_startup_collar_and_tiny_remaining_time()
{
    let mut maximum = 0.0_f64;
    let mut count = 0;
    for line in include_str!("fixtures/reduced-force-words.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(
            columns.len(),
            23,
            "one clock, three coordinates and 19 outputs"
        );
        let elapsed: u128 = columns[0].parse().unwrap();
        let target = 1u128 << 63;
        let clock =
            BenchmarkTime::new(TickClock::restore(-70, target, elapsed, target - elapsed).unwrap())
                .unwrap();
        let point = std::array::from_fn(|i| rational(columns[1 + i]));
        let actual = reduced_force::evaluate(point, clock).unwrap();
        let values = actual
            .velocity
            .into_iter()
            .chain([actual.pressure_raw])
            .chain(actual.force)
            .chain(actual.momentum_terms.into_iter().flatten());
        for (index, (value, text)) in values.zip(&columns[4..]).enumerate() {
            let expected = rational(text);
            let error = (value - expected).abs() / (1.0 + expected.abs());
            maximum = maximum.max(error);
            assert!(error<5e-12,"tick {elapsed}, point {point:?}, value {index}: {value} vs {expected}, scaled {error}");
        }
        count += 1;
    }
    assert_eq!(count, 84);
    println!("reduced force exact-word pointwise fixtures count={count} maximum_scaled_error={maximum:e}");
}

#[test]
fn flat_support_periodicity_nonfinite_refusals_and_axial_asymmetry_remain_explicit() {
    let active = time("1/256");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(reduced_force::evaluate([bad, 0.0, 0.0], active).is_err());
    }
    for (point, clock) in [
        ([0.03125, 0.125, -0.125], time("0")),
        ([0.5, 0.0, 0.0], active),
        ([0.5; 3], active),
    ] {
        let sample = reduced_force::evaluate(point, clock).unwrap();
        assert_eq!(sample.force, [0.0; 3]);
        assert_eq!(sample.velocity, [0.0; 3]);
        assert_eq!(sample.pressure_raw, 0.0);
        assert!(sample.root.is_none());
        assert_eq!(sample.root_residual, [0.0; 20]);
    }
    let point = [0.03125, -0.125, 0.125];
    let a = reduced_force::evaluate(point, active).unwrap();
    let b =
        reduced_force::evaluate([point[0] + 1.0, point[1] - 2.0, point[2] + 4.0], active).unwrap();
    assert_eq!(a.force.map(f64::to_bits), b.force.map(f64::to_bits));
    let report = a.root.unwrap();
    assert!(report.iterations > 0 && report.iterations <= 128);
    assert!(a.root_residual.iter().all(|x| x.is_finite()));
    let reflected = reduced_force::evaluate([point[0], point[1], -point[2]], active).unwrap();
    assert_ne!(a.force[2].to_bits(), reflected.force[2].to_bits());
}
