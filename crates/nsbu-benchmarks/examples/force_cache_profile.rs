//! Measure bounded v2 force cost and full coefficient identity; timings are informational.
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
const CAP: usize = 2 * 1024 * 1024;
fn main() -> Result<(), SolverError> {
    let dry = dry_run(&std::env::args().skip(1).collect::<Vec<_>>())?;
    for n in [8, 12, 16] {
        profile(n, dry)?;
    }
    Ok(())
}
fn dry_run(args: &[String]) -> Result<bool, SolverError> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--dry-run" => Ok(true),
        _ => Err(SolverError::InvalidPayload),
    }
}
fn profile(n: usize, dry: bool) -> Result<(), SolverError> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let sampled = Layout::new([3 * n / 2; 3])?;
    let limits = V2Force::preflight(domain, sampled)?;
    // Fixed small public grids: include output spectra and a conservative metadata/I/O allowance.
    let joint = limits.storage_bytes + 48 * domain.layout().half_len() + 65536;
    if joint > CAP {
        return Err(SolverError::ResourceLimit);
    }
    println!("plan case=similarity-mms-v2 grid={n} samples={} quantum=2^-10 target=8 ticks=[1,4,2,1] provider={limits:?} joint_bytes={joint} cap={CAP}; no trajectory is evolved",3*n/2);
    if dry {
        return Ok(());
    }
    let mut provider = V2Force::new(domain, sampled, limits.storage_bytes)?;
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    for tick in [1, 4, 2, 1] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick)?;
        let start = std::time::Instant::now();
        let report = provider.evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))?;
        let elapsed = start.elapsed().as_secs_f64();
        println!("grid={n} samples={} tick={tick} seconds={elapsed:.9} roots={} work={} bytes={} sha256={}",3*n/2,provider.last_root_iterations(),report.work_units,limits.storage_bytes,coefficient_hash(&output));
    }
    Ok(())
}
fn coefficient_hash(output: &[Vec<Complex64>; 3]) -> String {
    let mut hash = Sha256::new();
    for component in output {
        for v in component {
            hash.update(v.re.to_bits().to_le_bytes());
            hash.update(v.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}
#[test]
fn bounded_public_profile_checks_admission_and_nonmonotone_force_samples() {
    assert!(!dry_run(&[]).unwrap());
    assert!(dry_run(&["--dry-run".into()]).unwrap());
    assert!(dry_run(&["bad".into()]).is_err());
    assert!(dry_run(&["--dry-run".into(), "extra".into()]).is_err());
    for n in [8, 12, 16] {
        profile(n, true).unwrap();
    }
    profile(4, false).unwrap();
    assert!(matches!(profile(32, true), Err(SolverError::ResourceLimit)));
}
