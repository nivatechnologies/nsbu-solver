use nsbu_benchmarks::{
    fields::reference,
    regions::classify,
    scalar,
    time::BenchmarkTime,
};
use nsbu_solver::domain::TickClock;
use std::{env, hint::black_box, time::Instant};

const M: usize = 32;

fn main() -> Result<(), String> {
    let elapsed: u128 = env::args().nth(1).ok_or("ELAPSED")?.parse().map_err(debug)?;
    if ![512, 4096].contains(&elapsed) { return Err("ELAPSED must be 512 or 4096".into()); }
    let clock = TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).map_err(debug)?;
    let time = BenchmarkTime::new(clock).map_err(debug)?;
    let points = (0..M).flat_map(|i| (0..M).flat_map(move |j| (0..M).map(move |k| {
        [i as f64 / M as f64, j as f64 / M as f64, k as f64 / M as f64]
    })));
    let start = Instant::now();
    let mut scalar_roots = 0usize;
    let mut scalar_iterations = 0usize;
    let mut checksum = 0.0;
    for point in points.clone() {
        let value = scalar::evaluate(point, time).map_err(debug)?;
        if let Some(root) = value.root { scalar_roots += 1; scalar_iterations += root.iterations; }
        checksum += value.velocity[0];
    }
    let scalar_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut reference_roots = 0usize;
    let mut reference_iterations = 0usize;
    for point in points.clone() {
        let value = reference::evaluate(point, time).map_err(debug)?;
        if let Some(root) = value.root { reference_roots += 1; reference_iterations += root.iterations; }
        checksum += value.hessian[0][0][0];
    }
    let reference_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut classify_roots = 0usize;
    let mut classify_iterations = 0usize;
    for point in points {
        let value = classify(point, clock, 128).map_err(debug)?;
        if let Some(root) = value.root { classify_roots += 1; classify_iterations += root.iterations; }
        checksum += value.root.map_or(0.0, |root| root.value);
    }
    let classify_seconds = start.elapsed().as_secs_f64();
    println!("elapsed={elapsed} points={} scalar_seconds={scalar_seconds:.9} scalar_roots={scalar_roots} scalar_iterations={scalar_iterations} reference_seconds={reference_seconds:.9} reference_roots={reference_roots} reference_iterations={reference_iterations} classify_seconds={classify_seconds:.9} classify_roots={classify_roots} classify_iterations={classify_iterations} checksum={}", M.pow(3), black_box(checksum));
    Ok(())
}

fn debug(value: impl std::fmt::Debug) -> String { format!("{value:?}") }
