//! Fixed binary64 inputs and outputs for the bounded high-precision reference pilot.
use nsbu_benchmarks::{fields::reference, time::BenchmarkTime};
use nsbu_solver::domain::TickClock;

const POINTS: [(&str, [usize; 3]); 4] = [
    ("axis", [0, 0, 1]),
    ("collar", [5, 0, 0]),
    ("interior", [1, 1, 1]),
    ("exterior", [6, 0, 0]),
];
const CLOCKS: [u128; 3] = [0, 64, 128];

fn class(index: [usize; 3], elapsed: u128) -> &'static str {
    if elapsed == 0 {
        return "startup";
    }
    let centered = index.map(|value| {
        if value < 6 {
            value as isize
        } else {
            value as isize - 12
        }
    });
    let radius_numerator = centered
        .into_iter()
        .map(|value| value * value)
        .sum::<isize>();
    if radius_numerator * 2500 >= 441 * 144 {
        "exterior"
    } else {
        "active"
    }
}

fn flatten(sample: reference::ReferenceEvaluation) -> Vec<f64> {
    sample
        .velocity
        .into_iter()
        .chain(sample.gradient.into_iter().flatten())
        .chain(sample.hessian.into_iter().flatten().flatten())
        .chain(sample.vorticity)
        .collect()
}

fn fields(point: [f64; 3], time: BenchmarkTime) -> Vec<f64> {
    flatten(reference::evaluate(point, time).unwrap())
}

fn words(values: impl IntoIterator<Item = f64>) -> String {
    values
        .into_iter()
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}

fn row(prefix: &str, label: &str, index: [usize; 3], elapsed: u128) -> (&'static str, String) {
    let argument = index.map(|value| value as f64 / 12.0);
    let centered = argument.map(|value| if value >= 0.5 { value - 1.0 } else { value });
    let clock = TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap();
    let time = BenchmarkTime::new(clock).unwrap();
    assert_eq!(time.rounding_estimates()[0], 0.0);
    let category = class(index, elapsed);
    let values = fields(argument, time);
    assert_eq!(values.len(), 42);
    assert_eq!(values, fields(centered, time));
    if category != "active" {
        assert!(values.iter().all(|value| value.to_bits() == 0));
    }
    let line = format!(
        "{prefix}\t{label}\t{},{},{}\t{elapsed}\t{category}\t{}\t{}\t{:016x}\t{}",
        index[0],
        index[1],
        index[2],
        words(argument),
        words(centered),
        time.elapsed().to_bits(),
        words(values)
    );
    (category, line)
}

#[test]
fn emit_fixed_binary64_reference_rows() {
    let mut counts = [0usize; 3];
    for (label, index) in POINTS {
        for elapsed in CLOCKS {
            let (category, line) = row("ARITH_PILOT", label, index, elapsed);
            counts[match category {
                "startup" => 0,
                "exterior" => 1,
                "active" => 2,
                _ => unreachable!(),
            }] += 1;
            println!("{line}");
        }
    }
    assert_eq!(counts, [4, 2, 6]);
}

#[test]
fn emit_full_binary64_reference_rows() {
    let mut counts = [0usize; 3];
    let mut rows = 0usize;
    for elapsed in CLOCKS {
        for flat in 0..12usize.pow(3) {
            let index = [flat / 144, (flat / 12) % 12, flat % 12];
            let (category, line) = row("ARITH_FULL", "full", index, elapsed);
            counts[match category {
                "startup" => 0,
                "exterior" => 1,
                "active" => 2,
                _ => unreachable!(),
            }] += 1;
            rows += 1;
            println!("{line}");
        }
    }
    assert_eq!(rows, 5184);
    assert_eq!(counts, [1728, 2426, 1030]);
}
