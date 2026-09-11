//! Pointwise arithmetic comparison; neither evaluator is used to advance a PDE trajectory here.
use nsbu_benchmarks::{fields, reduced_force, time::BenchmarkTime, BenchmarkError};
use nsbu_solver::domain::TickClock;
type Evaluator = fn([f64; 3], BenchmarkTime) -> Result<[f64; 3], BenchmarkError>;
fn main() -> Result<(), BenchmarkError> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    profile(16, arguments(&args)?)
}
fn arguments(args: &[String]) -> Result<bool, BenchmarkError> {
    match args {
        [] => Ok(false),
        [arg] if arg == "--dry-run" => Ok(true),
        _ => Err(BenchmarkError::InvalidInput),
    }
}
fn original(point: [f64; 3], time: BenchmarkTime) -> Result<[f64; 3], BenchmarkError> {
    Ok(fields::evaluate(point, time)?.force)
}
fn reduced(point: [f64; 3], time: BenchmarkTime) -> Result<[f64; 3], BenchmarkError> {
    Ok(reduced_force::evaluate(point, time)?.force)
}
fn profile(n: usize, dry: bool) -> Result<(), BenchmarkError> {
    if ![4, 16].contains(&n) {
        return Err(BenchmarkError::InvalidInput);
    }
    println!("pointwise preflight n={n} samples={} clocks=[1,4,2,1] quantum=2^-10 target=8 force_calls={} root_iterations_per_call_max=128 formal_newton_steps=3 numerical_heap_bytes=0 stack_allowance_bytes=8388608",n*n*n,16*n*n*n);
    println!("case=similarity-mms-v2; Cartesian/reduced pointwise arithmetic comparison; no Fourier force sampling or PDE trajectory qualification");
    if dry {
        return Ok(());
    }
    for tick in [1, 4, 2, 1] {
        let time = BenchmarkTime::new(
            TickClock::restore(-10, 8, tick, 8 - tick).map_err(|_| BenchmarkError::InvalidInput)?,
        )?;
        measure(n, tick, time, "cartesian", original)?;
        measure(n, tick, time, "reduced", reduced)?;
        let error = compare(n, time)?;
        println!("comparison tick={tick} maximum_scaled_force_difference={error:e}");
        if error >= 5e-12 {
            return Err(BenchmarkError::ArithmeticResolution);
        }
    }
    Ok(())
}
fn point(n: usize, index: usize) -> [f64; 3] {
    [index / (n * n), (index / n) % n, index % n].map(|i| i as f64 / n as f64)
}
fn measure(
    n: usize,
    tick: u128,
    time: BenchmarkTime,
    label: &str,
    evaluator: Evaluator,
) -> Result<(), BenchmarkError> {
    let start = std::time::Instant::now();
    let mut checksum = [0.0; 3];
    for index in 0..n * n * n {
        let force = std::hint::black_box(evaluator(point(n, index), time)?);
        for (sum, value) in checksum.iter_mut().zip(force) {
            *sum += value;
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    println!("evaluator={label} tick={tick} seconds={seconds:.9} checksum={checksum:?}");
    Ok(())
}
fn compare(n: usize, time: BenchmarkTime) -> Result<f64, BenchmarkError> {
    let mut maximum = 0.0_f64;
    for index in 0..n * n * n {
        let point = point(n, index);
        for (a, b) in original(point, time)?
            .into_iter()
            .zip(reduced(point, time)?)
        {
            maximum = maximum.max((a - b).abs() / (1.0 + a.abs()));
        }
    }
    Ok(maximum)
}
#[test]
fn bounded_public_preflight_comparison_and_argument_refusals() {
    assert!(!arguments(&[]).unwrap());
    assert!(arguments(&["--dry-run".into()]).unwrap());
    assert!(arguments(&["bad".into()]).is_err());
    assert!(arguments(&["--dry-run".into(), "bad".into()]).is_err());
    assert!(profile(2, true).is_err());
    profile(4, true).unwrap();
    profile(4, false).unwrap();
}
