//! Optional sampled-force comparison; this profile evolves no PDE state.
use nsbu_benchmarks::{
    provider::{reduced::ReducedV2Force, V2Force},
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};

fn main() -> Result<(), SolverError> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    profile(16, arguments(&args)?)
}
fn arguments(args: &[String]) -> Result<bool, SolverError> {
    match args {
        [] => Ok(false),
        [arg] if arg == "--dry-run" => Ok(true),
        _ => Err(SolverError::InvalidPayload),
    }
}
fn profile(n: usize, dry: bool) -> Result<(), SolverError> {
    let (domain, sampled, old, new) = admit(n)?;
    if dry {
        return Ok(());
    }
    let mut cartesian = V2Force::new(domain, sampled, old.storage_bytes)?;
    let mut reduced = ReducedV2Force::new(domain, sampled, new.storage_bytes)?;
    let mut a: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut b = a.clone();
    for tick in [1, 4, 2, 1] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick)?;
        measure("cartesian", &mut cartesian, old, clock, &mut a)?;
        measure("reduced", &mut reduced, new, clock, &mut b)?;
        let error = a
            .iter()
            .flatten()
            .zip(b.iter().flatten())
            .fold(0.0_f64, |m, (a, b)| {
                m.max((*a - *b).l1_norm() / (1.0 + a.l1_norm()))
            });
        println!("comparison tick={tick} maximum_scaled_coefficient_difference={error:e}");
        if error >= 5e-12 {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
    }
    println!("force-only diagnostic; no trajectory speed or force-grid convergence claim; accepted_pde_windows=0");
    Ok(())
}

/// Admit both providers and all comparison storage before allocating either owner.
fn admit(n: usize) -> Result<(Domain, Layout, ForceLimits, ForceLimits), SolverError> {
    if ![4, 16].contains(&n) {
        return Err(SolverError::InvalidDomain);
    }
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let sampled = Layout::new([3 * n / 2; 3])?;
    let old = V2Force::preflight(domain, sampled)?;
    let new = ReducedV2Force::preflight(domain, sampled)?;
    let outputs = domain
        .layout()
        .half_len()
        .checked_mul(16 * 3 * 2)
        .ok_or(SolverError::SizeOverflow)?;
    let joint = old
        .storage_bytes
        .checked_add(new.storage_bytes)
        .and_then(|v| v.checked_add(outputs))
        .and_then(|v| v.checked_add(8 * 1024 * 1024 + 16 * 1024))
        .ok_or(SolverError::SizeOverflow)?;
    if joint > 32 * 1024 * 1024 {
        return Err(SolverError::ResourceLimit);
    }
    println!("case=similarity-mms-v2 sha256={CASE_SHA256} retained={n} sampled={} clocks=[1,4,2,1] quantum=2^-10 target=8",3*n/2);
    println!("joint_storage_bytes={joint} stack_allowance_bytes=8388608 profile_metadata_allowance_bytes=16384 cartesian={old:?} reduced={new:?} requests_per_provider=4 scalar_transforms_total=24");
    Ok((domain, sampled, old, new))
}
fn measure(
    provider_name: &str,
    provider: &mut impl PrescribedForce,
    limits: ForceLimits,
    clock: TickClock,
    output: &mut [Vec<Complex64>; 3],
) -> Result<(), SolverError> {
    let start = std::time::Instant::now();
    let work = provider.evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))?;
    let seconds = start.elapsed().as_secs_f64();
    let mut hash = Sha256::new();
    for value in output.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    println!(
        "provider={provider_name} tick={} seconds={seconds:.9} work={} transforms={} sha256={:x}",
        clock.elapsed(),
        work.work_units,
        work.scalar_transforms,
        hash.finalize()
    );
    Ok(())
}
#[test]
fn bounded_public_profile_and_argument_refusals() {
    assert!(!arguments(&[]).unwrap());
    assert!(arguments(&["--dry-run".into()]).unwrap());
    assert!(arguments(&["bad".into()]).is_err());
    assert!(arguments(&["--dry-run".into(), "bad".into()]).is_err());
    assert!(profile(2, true).is_err());
    profile(4, true).unwrap();
    profile(4, false).unwrap();
}
