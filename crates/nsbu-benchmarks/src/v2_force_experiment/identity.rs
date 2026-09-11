//! Streaming versioned identity for the exact-v2 force-sampling family.
use super::ForceFamilySettings;
use nsbu_solver::{verification::times::TestedTimes, SolverError};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 16] = b"NSBUV2FORCEFAM01";

pub(super) fn compute(
    settings: ForceFamilySettings,
    times: TestedTimes<'_>,
) -> Result<[u8; 32], SolverError> {
    encoded_len(times.as_slice().len())?;
    let mut hash = Sha256::new();
    hash.update(MAGIC);
    hash.update(crate::CASE_SHA256.as_bytes());
    put_usize(&mut hash, settings.grid);
    for value in settings.force_grids {
        put_usize(&mut hash, value);
    }
    put_usize(&mut hash, settings.workers);
    put_u128(&mut hash, settings.step_ticks);
    put_u64(&mut hash, method_word(settings.method));
    put_u128(&mut hash, settings.endpoint);
    for value in settings.tolerances.absolute {
        put_u64(&mut hash, value.to_bits());
    }
    for value in settings.tolerances.relative {
        put_u64(&mut hash, value.to_bits());
    }
    put_u64(&mut hash, settings.advective_limit.to_bits());
    put_usize(&mut hash, times.as_slice().len());
    for clock in times.as_slice() {
        hash.update(clock.exponent().to_le_bytes());
        put_u128(&mut hash, clock.target());
        put_u128(&mut hash, clock.elapsed());
        put_u128(&mut hash, clock.remaining());
    }
    Ok(hash.finalize().into())
}

pub(super) fn encoded_len(samples: usize) -> Result<usize, SolverError> {
    let fixed = MAGIC
        .len()
        .checked_add(crate::CASE_SHA256.len())
        .and_then(|n| n.checked_add(5 * 16 + 16 + 8 + 16 + 5 * 8 + 16))
        .ok_or(SolverError::SizeOverflow)?;
    fixed
        .checked_add(samples.checked_mul(52).ok_or(SolverError::SizeOverflow)?)
        .ok_or(SolverError::SizeOverflow)
}
fn method_word(method: nsbu_solver::integrators::method::Method) -> u64 {
    match method {
        nsbu_solver::integrators::method::Method::CoxMatthews => 0,
        nsbu_solver::integrators::method::Method::HochbruckOstermann => 1,
    }
}
fn put_usize(hash: &mut Sha256, value: usize) {
    hash.update((value as u128).to_le_bytes());
}
fn put_u128(hash: &mut Sha256, value: u128) {
    hash.update(value.to_le_bytes());
}
fn put_u64(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
