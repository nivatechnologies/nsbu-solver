//! Canonical identity for one immutable original-force slab table.
use super::{SharedForceClock, SharedForceTablePlan};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 16] = b"NSBUV2FORCETBL01";

pub(super) fn compute(plan: &SharedForceTablePlan<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(MAGIC);
    hash.update(crate::CASE_SHA256.as_bytes());
    put_layout(&mut hash, plan.settings().samples);
    put_usize(&mut hash, plan.settings().workers);
    for domain in plan.domains() {
        put_layout(&mut hash, domain.layout());
        for length in domain.lengths() {
            hash.update(length.to_bits().to_le_bytes());
        }
        hash.update(domain.viscosity().to_bits().to_le_bytes());
    }
    put_usize(&mut hash, plan.maximum_attempts());
    put_usize(&mut hash, plan.manifest().len());
    for request in plan.manifest() {
        put_clock(&mut hash, *request);
    }
    hash.finalize().into()
}

fn put_clock(hash: &mut Sha256, request: SharedForceClock) {
    hash.update(request.clock().exponent().to_le_bytes());
    hash.update(request.clock().target().to_le_bytes());
    hash.update(request.clock().elapsed().to_le_bytes());
    hash.update(request.clock().remaining().to_le_bytes());
    for count in request.maximum_copies() {
        put_usize(hash, count);
    }
}

fn put_layout(hash: &mut Sha256, layout: nsbu_solver::domain::Layout) {
    put_usize(hash, layout.half_len());
    for dimension in layout.dimensions() {
        put_usize(hash, dimension);
    }
}

fn put_usize(hash: &mut Sha256, value: usize) {
    hash.update((value as u128).to_le_bytes());
}
