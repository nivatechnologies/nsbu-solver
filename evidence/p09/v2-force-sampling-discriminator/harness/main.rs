use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, norms::Norms},
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::modal,
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};

const CLOCKS: [u128; 6] = [1920, 1984, 2016, 2032, 2047, 2048];
const CAP: usize = 1024 * 1024 * 1024;

fn main() -> Result<(), SolverError> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let dry_run = arguments.as_slice() == ["--dry-run"];
    if !dry_run && !arguments.is_empty() {
        return Err(SolverError::InvalidPayload);
    }
    let domain = Domain::new([24; 3], [1.0; 3], 1.0)?;
    let low = ForceSettings { samples: Layout::new([24; 3])?, workers: 12 };
    let high = low.double_grid()?;
    let low_limits = low.limits(domain)?;
    let high_limits = high.limits(domain)?;
    let field_bytes = 3usize
        .checked_mul(domain.layout().half_len()).and_then(|v| v.checked_mul(16))
        .ok_or(SolverError::SizeOverflow)?;
    let fields = field_bytes.checked_mul(3).ok_or(SolverError::SizeOverflow)?;
    let stored = low_limits.storage_bytes.checked_add(high_limits.storage_bytes)
        .and_then(|v| v.checked_add(fields)).and_then(|v| v.checked_add(4096))
        .ok_or(SolverError::SizeOverflow)?;
    let calls = CLOCKS.len();
    let work = low_limits.work_units.checked_add(high_limits.work_units)
        .and_then(|v| v.checked_mul(calls)).ok_or(SolverError::SizeOverflow)?;
    let transforms = low_limits.scalar_transforms.checked_add(high_limits.scalar_transforms)
        .and_then(|v| v.checked_mul(calls)).ok_or(SolverError::SizeOverflow)?;
    let comparison_work = ComparisonPlan::new(domain, domain)?.work_units()
        .checked_mul(6).and_then(|v| v.checked_mul(calls)).ok_or(SolverError::SizeOverflow)?;
    let projection_visits = domain.layout().half_len().checked_mul(6)
        .and_then(|v| v.checked_mul(calls)).ok_or(SolverError::SizeOverflow)?;
    println!("preflight source=N24 retained_half={} force_low=M24_workers12 {:?} force_high=M48_workers12 {:?} fields_bytes={} joint_bytes={} cap={} provider_work={} provider_transforms={} comparison_work={} projection_coefficient_visits={} clocks={CLOCKS:?}", domain.layout().half_len(), low_limits, high_limits, fields, stored, CAP, work, transforms, comparison_work, projection_visits);
    if stored > CAP { return Err(SolverError::ResourceLimit); }
    if dry_run { return Ok(()); }
    let mut a = low.build(domain, CAP)?;
    let mut b = high.build(domain, CAP)?;
    let zero = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut left = zero.clone();
    let mut right = zero.clone();
    let compare = ComparisonPlan::new(domain, domain)?;
    for elapsed in CLOCKS {
        let clock = TickClock::restore(-20, 8192, elapsed, 8192 - elapsed)?;
        let aw = a.evaluate(clock, a.limits().ok_or(SolverError::UnknownProviderCost)?, left.each_mut().map(Vec::as_mut_slice))?;
        let bw = b.evaluate(clock, b.limits().ok_or(SolverError::UnknownProviderCost)?, right.each_mut().map(Vec::as_mut_slice))?;
        let difference = compare.compare(left.each_ref().map(Vec::as_slice), right.each_ref().map(Vec::as_slice))?;
        let left_norm = compare.compare(zero.each_ref().map(Vec::as_slice), left.each_ref().map(Vec::as_slice))?.full;
        let right_norm = compare.compare(zero.each_ref().map(Vec::as_slice), right.each_ref().map(Vec::as_slice))?.full;
        let raw_hashes = [digest(&left), digest(&right)];
        project(domain, &mut left)?;
        project(domain, &mut right)?;
        let projected_difference = compare.compare(left.each_ref().map(Vec::as_slice), right.each_ref().map(Vec::as_slice))?.full;
        let projected_left = compare.compare(zero.each_ref().map(Vec::as_slice), left.each_ref().map(Vec::as_slice))?.full;
        let projected_right = compare.compare(zero.each_ref().map(Vec::as_slice), right.each_ref().map(Vec::as_slice))?.full;
        emit(elapsed, aw, bw, difference.full, left_norm, right_norm, raw_hashes, projected_difference, projected_left, projected_right, [digest(&left), digest(&right)]);
    }
    Ok(())
}
fn project(domain: Domain, field: &mut [Vec<Complex64>; 3]) -> Result<(), SolverError> {
    for index in 0..domain.layout().half_len() {
        let position = domain.layout().position(index)?;
        let value = if domain.layout().is_nyquist(position)? { [Complex64::new(0.0, 0.0); 3] } else {
            modal::project(modal::wavevector(domain, domain.layout().mode(position)?)?, std::array::from_fn(|axis| field[axis][index]))?
        };
        for (axis, value) in value.into_iter().enumerate() { field[axis][index] = value; }
    }
    Ok(())
}
fn digest(field: &[Vec<Complex64>; 3]) -> [u8; 32] {
    let mut hash = Sha256::new();
    for component in field { for value in component { hash.update(value.re.to_bits().to_le_bytes()); hash.update(value.im.to_bits().to_le_bytes()); } }
    hash.finalize().into()
}
fn emit(elapsed: u128, low: nsbu_solver::integrators::forcing::ForceWork, high: nsbu_solver::integrators::forcing::ForceWork, difference: Norms, m24: Norms, m48: Norms, raw_hashes: [[u8; 32]; 2], projected_difference: Norms, projected_m24: Norms, projected_m48: Norms, projected_hashes: [[u8; 32]; 2]) {
 println!("clock={elapsed} low_work={low:?} high_work={high:?} raw_difference={difference:?} raw_m24={m24:?} raw_m48={m48:?} raw_hashes={raw_hashes:02x?} projected_difference={projected_difference:?} projected_m24={projected_m24:?} projected_m48={projected_m48:?} projected_hashes={projected_hashes:02x?}");
}
