//! Compare persistent parallel workers with the serial exact-v2 force under explicit joint caps.
use nsbu_benchmarks::provider::{parallel::ParallelV2Force, V2Force};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
const CAP: usize = 256 * 1024 * 1024;
fn main() -> Result<(), SolverError> {
    let (n, dry) = arguments(&std::env::args().skip(1).collect::<Vec<_>>())?;
    profile(n, dry)
}
fn arguments(args: &[String]) -> Result<(usize, bool), SolverError> {
    let (n, dry) = match args {
        [] => (16, false),
        [flag] if flag == "--dry-run" => (16, true),
        [n] => (n.parse().map_err(|_| SolverError::InvalidPayload)?, false),
        [n, flag] if flag == "--dry-run" => {
            (n.parse().map_err(|_| SolverError::InvalidPayload)?, true)
        }
        _ => return Err(SolverError::InvalidPayload),
    };
    if ![4, 8, 16, 32, 64].contains(&n) {
        return Err(SolverError::InvalidPayload);
    }
    Ok((n, dry))
}
fn profile(n: usize, dry: bool) -> Result<(), SolverError> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([3 * n / 2; 3])?;
    let serial = V2Force::preflight(domain, samples)?;
    let output_bytes = domain.layout().half_len() * 48;
    // Preflight every alternative before constructing either provider or output.
    for workers in [1, 2, 4, 8, 16, 32]
        .into_iter()
        .filter(|w| *w <= samples.dimensions()[2])
    {
        let limits = ParallelV2Force::preflight(domain, samples, workers)?;
        let joint = serial.storage_bytes + limits.storage_bytes + output_bytes + 65536;
        if joint > CAP {
            return Err(SolverError::ResourceLimit);
        }
        println!("plan grid={n} samples={} workers={workers} joint_bytes={joint} cap={CAP} force={limits:?}",samples.dimensions()[2]);
    }
    println!("case=similarity-mms-v2 quantum=2^-10 target=8 ticks=[1,4,2,1]; force-only diagnostic, no integrated state or accepted PDE window");
    if dry {
        return Ok(());
    }
    execute(domain, samples, serial)
}
fn execute(domain: Domain, samples: Layout, serial_limits: ForceLimits) -> Result<(), SolverError> {
    let mut serial = V2Force::new(domain, samples, serial_limits.storage_bytes)?;
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut reference: [String; 4] = std::array::from_fn(|_| String::new());
    for (slot, tick) in reference.iter_mut().zip([1, 4, 2, 1]) {
        *slot = measure(&mut serial, serial_limits, &mut output, 0, tick)?;
    }
    for workers in [1, 2, 4, 8, 16, 32]
        .into_iter()
        .filter(|w| *w <= samples.dimensions()[2])
    {
        compare_parallel(domain, samples, workers, &mut output, &reference)?;
    }

    Ok(())
}
fn compare_parallel(
    domain: Domain,
    samples: Layout,
    workers: usize,
    output: &mut [Vec<Complex64>; 3],
    reference: &[String; 4],
) -> Result<(), SolverError> {
    let limits = ParallelV2Force::preflight(domain, samples, workers)?;
    let mut parallel = ParallelV2Force::new(domain, samples, workers, limits.storage_bytes)?;
    for (expected, tick) in reference.iter().zip([1, 4, 2, 1]) {
        let actual = measure(&mut parallel, limits, output, workers, tick)?;
        if &actual != expected {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
    }
    Ok(())
}
fn measure(
    provider: &mut impl PrescribedForce,
    limits: ForceLimits,
    output: &mut [Vec<Complex64>; 3],
    workers: usize,
    tick: u128,
) -> Result<String, SolverError> {
    let clock = TickClock::restore(-10, 8, tick, 8 - tick)?;
    let start = std::time::Instant::now();
    let work = provider.evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))?;
    let seconds = start.elapsed().as_secs_f64();
    let mut hash = Sha256::new();
    for value in output.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    let hash = format!("{:x}", hash.finalize());
    println!(
        "workers={workers} tick={tick} seconds={seconds:.9} work={} transforms={} sha256={hash}",
        work.work_units, work.scalar_transforms
    );
    Ok(hash)
}
#[test]
fn public_profile_arguments_preflight_and_complete_small_execution() {
    assert_eq!(arguments(&[]).unwrap(), (16, false));
    assert_eq!(arguments(&["--dry-run".into()]).unwrap(), (16, true));
    assert_eq!(arguments(&["4".into()]).unwrap(), (4, false));
    assert_eq!(
        arguments(&["8".into(), "--dry-run".into()]).unwrap(),
        (8, true)
    );
    for args in [
        vec!["bad"],
        vec!["bad", "--dry-run"],
        vec!["4", "bad"],
        vec!["5"],
        vec!["4", "--dry-run", "bad"],
    ] {
        assert!(arguments(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
    }
    assert!(matches!(
        profile(128, true),
        Err(SolverError::ResourceLimit)
    ));
    profile(4, true).unwrap();
    profile(4, false).unwrap();
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let mut output =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let wrong =
        std::array::from_fn(|_| String::from("deliberately incorrect complete coefficient hash"));
    assert_eq!(
        compare_parallel(domain, Layout::new([6; 3]).unwrap(), 1, &mut output, &wrong),
        Err(SolverError::ArithmeticResolutionLimited)
    );
}
