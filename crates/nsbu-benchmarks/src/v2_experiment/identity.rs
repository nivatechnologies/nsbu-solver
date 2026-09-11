//! Canonical, bounded identity for an exact-v2 refinement family.
//!
//! Version 1 starts with `NSBUV2FAMILY0001`, then the 64 ASCII case-hash bytes.
//! Grids, steps, force half-length/dimensions, workers and endpoint are u128 LE.
//! Four tolerance words and the advective guard are binary64 u64 LE, followed
//! by the u128 time count and each clock (i32 exponent, u128 target/elapsed/remaining).
//! The fixed unit cube, viscosity one and CM/HO branch schedule are versioned semantics.
//! The encoding uses little-endian integer words and
//! the raw IEEE-754 words of floating point settings.  It binds `CASE_SHA256`,
//! every branch setting, and the complete exact tested-time manifest.  Hashing
//! is streaming and performs no allocation.
use super::FamilySettings;
use nsbu_solver::{verification::times::TestedTimes, SolverError};
use sha2::{Digest, Sha256};
const MAGIC: &[u8; 16] = b"NSBUV2FAMILY0001";

pub(super) fn compute(
    settings: FamilySettings,
    times: TestedTimes<'_>,
) -> Result<[u8; 32], SolverError> {
    encoded_len(times.as_slice().len())?;
    let mut hash = Sha256::new();
    hash.update(MAGIC);
    hash.update(crate::CASE_SHA256.as_bytes());
    for value in settings.grids {
        put_usize(&mut hash, value);
    }
    for value in settings.steps {
        put_u128(&mut hash, value);
    }
    put_usize(&mut hash, settings.force.samples.half_len());
    for value in settings.force.samples.dimensions() {
        put_usize(&mut hash, value);
    }
    put_usize(&mut hash, settings.force.workers);
    put_u128(&mut hash, settings.endpoint);
    for value in settings.tolerances.absolute.iter().copied() {
        put_u64(&mut hash, value.to_bits());
    }
    for value in settings.tolerances.relative.iter().copied() {
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

pub(super) fn encoded_len(time_count: usize) -> Result<usize, SolverError> {
    let fixed = MAGIC
        .len()
        .checked_add(crate::CASE_SHA256.len())
        .and_then(|n| n.checked_add(3 * 16 + 3 * 16 + 16 + 3 * 16 + 16 + 16 + 4 * 8 + 8 + 16))
        .ok_or(SolverError::SizeOverflow)?;
    fixed
        .checked_add(
            time_count
                .checked_mul(52)
                .ok_or(SolverError::SizeOverflow)?,
        )
        .ok_or(SolverError::SizeOverflow)
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
