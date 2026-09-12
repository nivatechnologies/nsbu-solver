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
    spectral::{FftBackend, FftCatalog},
    SolverError,
};
pub(super) fn limits(
    domain: Domain,
    samples: Layout,
    workers: usize,
) -> Result<ForceLimits, SolverError> {
    limits_inner(domain, samples, workers, None)
}

pub(super) fn limits_with_catalog(
    domain: Domain,
    samples: Layout,
    workers: usize,
    catalog: &FftCatalog,
) -> Result<ForceLimits, SolverError> {
    limits_inner(domain, samples, workers, Some(catalog))
}

pub(super) fn limits_with_fft_backend(
    domain: Domain,
    samples: Layout,
    workers: usize,
    backend: FftBackend,
) -> Result<ForceLimits, SolverError> {
    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let original = V2Force::preflight_with_fft_backend(domain, samples, backend)?;
    limits_parts(samples, workers, original)
}

fn limits_inner(
    domain: Domain,
    samples: Layout,
    workers: usize,
    catalog: Option<&FftCatalog>,
) -> Result<ForceLimits, SolverError> {
    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let original = match catalog {
        Some(catalog) => V2Force::preflight_with_catalog(domain, samples, catalog)?,
        None => V2Force::preflight(domain, samples)?,
    };
    limits_parts(samples, workers, original)
}

fn limits_parts(
    samples: Layout,
    workers: usize,
    original: ForceLimits,
) -> Result<ForceLimits, SolverError> {
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
    use crate::provider::reduced::ReducedV2Force;

    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let serial = ReducedV2Force::preflight(domain, samples)?;
    reduced_limits_parts(samples, workers, serial)
}

pub(in crate::provider) fn reduced_limits_with_fft_backend(
    domain: Domain,
    samples: Layout,
    workers: usize,
    backend: FftBackend,
) -> Result<ForceLimits, SolverError> {
    use crate::provider::reduced::ReducedV2Force;
    if workers == 0 || workers > 128 || workers > samples.dimensions()[2] {
        return Err(SolverError::InvalidPayload);
    }
    let serial = ReducedV2Force::preflight_with_fft_backend(domain, samples, backend)?;
    reduced_limits_parts(samples, workers, serial)
}

fn reduced_limits_parts(
    samples: Layout,
    workers: usize,
    serial: ForceLimits,
) -> Result<ForceLimits, SolverError> {
    use crate::{
        provider::parallel_reduced::ParallelReducedV2Force,
        reduced_force::axial::AxialRoot as ReducedAxialRoot,
    };
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
