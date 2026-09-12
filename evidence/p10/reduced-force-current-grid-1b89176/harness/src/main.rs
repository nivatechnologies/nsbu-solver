use nsbu_benchmarks::provider::{parallel::ParallelV2Force, reduced::ReducedV2Force};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

const CAP: usize = 8 * 1024 * 1024 * 1024;

fn main() -> Result<(), SolverError> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [n, m, workers, clock] = args.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    execute(parse(n)?, parse(m)?, parse(workers)?, parse(clock)?)
}

fn execute(n: usize, m: usize, workers: usize, elapsed: u128) -> Result<(), SolverError> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([m; 3])?;
    let original_limits = ParallelV2Force::preflight(domain, samples, workers)?;
    let reduced_limits = ReducedV2Force::preflight(domain, samples)?;
    let output_bytes = domain
        .layout()
        .half_len()
        .checked_mul(96)
        .ok_or(SolverError::SizeOverflow)?;
    let joint = original_limits
        .storage_bytes
        .checked_add(reduced_limits.storage_bytes)
        .and_then(|value| value.checked_add(output_bytes))
        .and_then(|value| value.checked_add(65_536))
        .ok_or(SolverError::SizeOverflow)?;
    println!("preflight source={} n={n} m={m} workers={workers} clock={elapsed} joint_bytes={joint} cap={CAP} original={original_limits:?} reduced={reduced_limits:?}", env!("RUN_SOURCE"));
    if joint > CAP {
        return Err(SolverError::ResourceLimit);
    }
    let mut original =
        ParallelV2Force::new(domain, samples, workers, original_limits.storage_bytes)?;
    let mut reduced = ReducedV2Force::new(domain, samples, reduced_limits.storage_bytes)?;
    let mut a: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut b = a.clone();
    let clock = TickClock::restore(-20, 8192, elapsed, 8192 - elapsed)?;
    measure(
        "original_parallel",
        &mut original,
        original_limits,
        clock,
        &mut a,
    )?;
    measure(
        "reduced_serial",
        &mut reduced,
        reduced_limits,
        clock,
        &mut b,
    )?;
    let report = ComparisonPlan::new(domain, domain)?.compare(
        a.each_ref().map(Vec::as_slice),
        b.each_ref().map(Vec::as_slice),
    )?;
    let maximum_scaled = a
        .iter()
        .flatten()
        .zip(b.iter().flatten())
        .fold(0.0_f64, |maximum, (left, right)| {
            maximum.max((*left - *right).l1_norm() / (1.0 + left.l1_norm()))
        });
    println!("terminal=complete comparison={report:?} maximum_scaled_coefficient_difference={maximum_scaled:e}");
    Ok(())
}

fn measure(
    label: &str,
    provider: &mut impl PrescribedForce,
    limits: ForceLimits,
    clock: TickClock,
    output: &mut [Vec<Complex64>; 3],
) -> Result<(), SolverError> {
    let started = Instant::now();
    let work = provider.evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))?;
    let mut hash = Sha256::new();
    for coefficient in output.iter().flatten() {
        hash.update(coefficient.re.to_bits().to_le_bytes());
        hash.update(coefficient.im.to_bits().to_le_bytes());
    }
    println!(
        "provider={label} seconds={:.9} work={work:?} sha256={:x}",
        started.elapsed().as_secs_f64(),
        hash.finalize()
    );
    Ok(())
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, SolverError> {
    value.parse().map_err(|_| SolverError::InvalidPayload)
}
