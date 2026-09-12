//! Allocation-free storage admission for the original FFT owner and persistent worker planes.
use super::{
    pool::Pool,
    worker::{Worker, STACK_BYTES, THREAD_ALLOWANCE},
    ParallelV2Force,
};
use crate::{fields::axial::AxialRoot, provider::V2Force};
use nsbu_solver::{
    domain::{Domain, Layout},
    integrators::forcing::ForceLimits,
    SolverError,
};
pub(super) fn limits(
    domain: Domain,
    samples: Layout,
    workers: usize,
) -> Result<ForceLimits, SolverError> {
    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let original = V2Force::preflight(domain, samples)?;
    let physical = samples
        .real_len()
        .checked_mul(24)
        .ok_or(SolverError::SizeOverflow)?;
    let roots = samples.dimensions()[2]
        .checked_mul(std::mem::size_of::<Option<AxialRoot>>())
        .ok_or(SolverError::SizeOverflow)?;
    // One pool vector, five allocations per worker, plus native-thread/TLS/stack allowance.
    let worker_storage = Worker::metadata_bytes() + 5 * 64 + STACK_BYTES + THREAD_ALLOWANCE;
    let threads = workers
        .checked_mul(worker_storage)
        .ok_or(SolverError::SizeOverflow)?;
    let storage_bytes = [
        physical,
        roots,
        threads,
        64,
        std::mem::size_of::<Pool>(),
        std::mem::size_of::<ParallelV2Force>(),
    ]
    .into_iter()
    .try_fold(original.storage_bytes, |total, bytes| {
        total.checked_add(bytes).ok_or(SolverError::SizeOverflow)
    })?;
    Ok(ForceLimits {
        storage_bytes,
        ..original
    })
}

pub(in crate::provider) fn reduced_limits(
    domain: Domain,
    samples: Layout,
    workers: usize,
) -> Result<ForceLimits, SolverError> {
    use crate::{
        provider::{parallel_reduced::ParallelReducedV2Force, reduced::ReducedV2Force},
        reduced_force::axial::AxialRoot as ReducedAxialRoot,
    };

    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let serial = ReducedV2Force::preflight(domain, samples)?;
    let physical = samples
        .real_len()
        .checked_mul(24)
        .ok_or(SolverError::SizeOverflow)?;
    let roots = samples.dimensions()[2]
        .checked_mul(std::mem::size_of::<Option<ReducedAxialRoot>>())
        .ok_or(SolverError::SizeOverflow)?;
    let worker_storage = Worker::metadata_bytes() + 5 * 64 + STACK_BYTES + THREAD_ALLOWANCE;
    let threads = workers
        .checked_mul(worker_storage)
        .ok_or(SolverError::SizeOverflow)?;
    let storage_bytes = [
        physical,
        roots,
        threads,
        64,
        std::mem::size_of::<Pool>(),
        std::mem::size_of::<ParallelReducedV2Force>(),
    ]
    .into_iter()
    .try_fold(serial.storage_bytes, |total, bytes| {
        total.checked_add(bytes).ok_or(SolverError::SizeOverflow)
    })?;
    Ok(ForceLimits {
        storage_bytes,
        ..serial
    })
}
