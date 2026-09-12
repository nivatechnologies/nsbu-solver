use nsbu_benchmarks::{
    provider::{
        parallel::ParallelV2Force, parallel_reduced::ParallelReducedV2Force,
        reduced::ReducedV2Force,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{validate_spectrum, Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

const CAP: usize = 8 * 1024 * 1024 * 1024;
const METADATA_BYTES: usize = 65_536;

fn main() -> Result<(), SolverError> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [n, m, workers, flag @ ..] = args.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    let dry = match flag {
        [] => false,
        [value] if value == "--dry-run" => true,
        _ => return Err(SolverError::InvalidPayload),
    };
    execute(parse(n)?, parse(m)?, parse(workers)?, dry)
}

fn execute(n: usize, m: usize, workers: usize, dry: bool) -> Result<(), SolverError> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([m; 3])?;
    let original = ParallelV2Force::preflight(domain, samples, workers)?;
    let reduced_serial = ReducedV2Force::preflight(domain, samples)?;
    let reduced_parallel = ParallelReducedV2Force::preflight(domain, samples, workers)?;
    let output_bytes = domain
        .layout()
        .half_len()
        .checked_mul(16 * 3 * 3)
        .ok_or(SolverError::SizeOverflow)?;
    let peak_owner = original
        .storage_bytes
        .max(reduced_serial.storage_bytes)
        .max(reduced_parallel.storage_bytes);
    let peak = peak_owner
        .checked_add(output_bytes)
        .and_then(|value| value.checked_add(METADATA_BYTES))
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} retained=[{n},{n},{n}] sampled=[{m},{m},{m}] workers={workers} quantum=-20 target=8192 clock=4096 cap={CAP} sequential_owner_peak_bytes={peak} retained_outputs_bytes={output_bytes} metadata_bytes={METADATA_BYTES} original={original:?} reduced_serial={reduced_serial:?} reduced_parallel={reduced_parallel:?}",
        env!("RUN_SOURCE")
    );
    if peak > CAP {
        return Err(SolverError::ResourceLimit);
    }
    if dry {
        return Ok(());
    }

    let clock = TickClock::restore(-20, 8192, 4096, 4096)?;
    let mut original_output = output(domain)?;
    let mut serial_output = output(domain)?;
    let mut parallel_output = output(domain)?;

    {
        let mut provider =
            ParallelV2Force::new(domain, samples, workers, original.storage_bytes)?;
        measure(
            "original_parallel",
            &mut provider,
            original,
            clock,
            &mut original_output,
        )?;
    }
    {
        let mut provider = ReducedV2Force::new(domain, samples, reduced_serial.storage_bytes)?;
        measure(
            "reduced_serial",
            &mut provider,
            reduced_serial,
            clock,
            &mut serial_output,
        )?;
    }
    {
        let mut provider = ParallelReducedV2Force::new(
            domain,
            samples,
            workers,
            reduced_parallel.storage_bytes,
        )?;
        let identity = provider.identity();
        println!("parallel_reduced_identity={identity:?}");
        measure(
            "reduced_parallel",
            &mut provider,
            reduced_parallel,
            clock,
            &mut parallel_output,
        )?;
    }

    validate(domain, &original_output)?;
    validate(domain, &serial_output)?;
    validate(domain, &parallel_output)?;
    if parallel_output != serial_output {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    let report = ComparisonPlan::new(domain, domain)?.compare(
        original_output.each_ref().map(Vec::as_slice),
        serial_output.each_ref().map(Vec::as_slice),
    )?;
    let maximum_scaled = original_output
        .iter()
        .flatten()
        .zip(serial_output.iter().flatten())
        .fold(0.0_f64, |maximum, (left, right)| {
            maximum.max((*left - *right).l1_norm() / (1.0 + left.l1_norm()))
        });
    println!(
        "terminal=complete serial_parallel_reduced_bit_equal=true full_spectra_valid=true original_reduced_comparison={report:?} maximum_scaled_coefficient_difference={maximum_scaled:e} arithmetic_identity=separate"
    );
    Ok(())
}

fn output(domain: Domain) -> Result<[Vec<Complex64>; 3], SolverError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(domain.layout().half_len())
        .map_err(|_| SolverError::AllocationFailed)?;
    output.resize(domain.layout().half_len(), Complex64::new(0.0, 0.0));
    Ok([output.clone(), output.clone(), output])
}

fn measure(
    label: &str,
    provider: &mut impl PrescribedForce,
    limits: ForceLimits,
    clock: TickClock,
    output: &mut [Vec<Complex64>; 3],
) -> Result<ForceWork, SolverError> {
    let started = Instant::now();
    let work = provider.evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))?;
    let seconds = started.elapsed().as_secs_f64();
    println!(
        "provider={label} seconds={seconds:.9} work={work:?} sha256={}",
        fingerprint(output)
    );
    Ok(work)
}

fn validate(domain: Domain, output: &[Vec<Complex64>; 3]) -> Result<(), SolverError> {
    for component in output {
        if component.iter().any(|value| !value.is_finite()) {
            return Err(SolverError::InvalidSpectrum);
        }
        validate_spectrum(domain.layout(), component, 1e-12)?;
    }
    Ok(())
}

fn fingerprint(values: &[Vec<Complex64>; 3]) -> String {
    let mut hash = Sha256::new();
    for value in values.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, SolverError> {
    value.parse().map_err(|_| SolverError::InvalidPayload)
}
