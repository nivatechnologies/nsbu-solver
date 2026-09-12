use super::{W3FftMode, STACK_BYTES, WIDTH};
use crate::{
    domain::Layout,
    spectral::{FftBackend, FftPlan},
    Complex64, SolverError,
};
use std::mem::size_of;

const THREAD_ALLOWANCE: usize = 64 * 1024;
const ALLOCATION_ALLOWANCE: usize = 64;
const WORKER_METADATA_ALLOWANCE: usize = 944;

pub(super) fn additional(
    layout: Layout,
    backend: FftBackend,
    mode: W3FftMode,
) -> Result<usize, SolverError> {
    admit_layout(layout)?;
    if backend != FftBackend::RustFft6_4_1AvxFma {
        return Err(SolverError::InvalidPayload);
    }
    let workspace = FftPlan::reservation_with_shared_backend(layout, backend)?;
    let spectral = layout
        .half_len()
        .checked_mul(size_of::<Complex64>())
        .ok_or(SolverError::SizeOverflow)?;
    let physical = layout
        .real_len()
        .checked_mul(size_of::<f64>())
        .ok_or(SolverError::SizeOverflow)?;
    let lanes = match mode {
        W3FftMode::Forward => 2usize
            .checked_mul(workspace)
            .and_then(|n| n.checked_add(2 * spectral))
            .and_then(|n| n.checked_add(2 * ALLOCATION_ALLOWANCE)),
        W3FftMode::Bidirectional => 2usize
            .checked_mul(workspace)
            .and_then(|n| n.checked_add(2 * spectral))
            .and_then(|n| n.checked_add(2 * physical))
            .and_then(|n| n.checked_add(4 * ALLOCATION_ALLOWANCE)),
    }
    .ok_or(SolverError::SizeOverflow)?;
    let worker = STACK_BYTES
        .checked_add(THREAD_ALLOWANCE)
        .and_then(|n| n.checked_add(WORKER_METADATA_ALLOWANCE))
        .ok_or(SolverError::SizeOverflow)?;
    lanes
        .checked_add(WIDTH * worker)
        .and_then(|n| n.checked_add(ALLOCATION_ALLOWANCE))
        .ok_or(SolverError::SizeOverflow)
}

fn admit_layout(layout: Layout) -> Result<(), SolverError> {
    let dimensions = layout.dimensions();
    if matches!(
        dimensions,
        [6, 6, 6] | [384, 384, 384] | [512, 512, 512] | [576, 576, 576]
    ) {
        return Ok(());
    }
    Err(SolverError::InvalidPayload)
}
