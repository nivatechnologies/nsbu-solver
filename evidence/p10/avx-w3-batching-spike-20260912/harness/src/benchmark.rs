use crate::{
    controls,
    model::{Operation, SerialBatch},
    pool::{reservation, ParallelBatch, Reservation},
    records::Timings,
    util::{debug, worker_debug, LENGTHS},
    GLOBAL,
};
use nsbu_solver::{
    domain::Layout,
    spectral::{FftBackend, FftCatalog},
};
use stats_alloc::Region;
use std::time::{Duration, Instant};

pub(crate) fn execute() -> Result<(), String> {
    emit_identity();
    let catalog = catalog()?;
    controls::validate(&catalog)?;
    profile_all(&catalog)?;
    println!("terminal=profile-complete");
    Ok(())
}

fn emit_identity() {
    println!(
        "identity source={} backend=rustfft-6.4.1-avx-avx2-fma width=3 runtime_avx={} runtime_avx2={} runtime_fma={} publication=drain_then_lane_order_swap",
        option_env!("SOURCE_ID").unwrap_or("uncommitted-spike"),
        std::is_x86_feature_detected!("avx"),
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("fma"),
    );
}

fn catalog() -> Result<FftCatalog, String> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().map_err(debug)?;
    let catalog_bytes = FftCatalog::reservation(backend).map_err(debug)?;
    FftCatalog::new(backend, catalog_bytes).map_err(debug)
}

fn profile_all(catalog: &FftCatalog) -> Result<(), String> {
    for length in LENGTHS {
        profile(length, catalog)?;
    }
    Ok(())
}

fn profile(length: usize, catalog: &FftCatalog) -> Result<(), String> {
    let (mut serial, mut parallel) = profile_owners(length, catalog)?;
    measure_profile(length, &mut serial, &mut parallel)
}

fn profile_owners(
    length: usize,
    catalog: &FftCatalog,
) -> Result<(SerialBatch, ParallelBatch), String> {
    let layout = Layout::new([length; 3]).map_err(debug)?;
    let bytes = reservation(layout)?;
    emit_reservation(length, &bytes);
    require_cap_refusal(length, layout, catalog, bytes.parallel_profile_bytes)?;
    let serial = SerialBatch::new(layout, catalog)?;
    let parallel = ParallelBatch::new(layout, catalog, bytes.parallel_profile_bytes)?;
    Ok((serial, parallel))
}

fn measure_profile(
    length: usize,
    serial: &mut SerialBatch,
    parallel: &mut ParallelBatch,
) -> Result<(), String> {
    warm_and_check_allocations(length, serial, parallel)?;
    let medians = interleaved_timings(length, serial, parallel)?.medians();
    let dispatch = timed_dispatch(parallel)?;
    let repeats = copy_repeats(length);
    let copy_forward = timed_copy(parallel, Operation::Forward, repeats)?;
    let copy_inverse = timed_copy(parallel, Operation::Inverse, repeats)?;
    let output_hash = validate_outputs(length, serial, parallel)?;
    medians.emit_summary(length, copy_forward, copy_inverse, dispatch, &output_hash);
    Ok(())
}

fn copy_repeats(length: usize) -> usize {
    if length <= 384 {
        3
    } else {
        1
    }
}

fn emit_reservation(length: usize, bytes: &Reservation) {
    println!(
        "reservation n={length} catalog_bytes={} scalar_workspace_bytes={} physical_lane_bytes={} spectral_lane_bytes={} serial_harness_bytes={} parallel_owner_bytes={} copy_adapter_bytes={} parallel_profile_bytes={} production_forward_additional_bytes={} production_inverse_additional_bytes={} production_bidirectional_additional_bytes={} stack_bytes_each=2097152 thread_allowance_each=65536 allocation_allowance_each=64",
        bytes.catalog_bytes,
        bytes.scalar_workspace_bytes,
        bytes.physical_lane_bytes,
        bytes.spectral_lane_bytes,
        bytes.serial_harness_bytes,
        bytes.parallel_owner_bytes,
        bytes.copy_adapter_bytes,
        bytes.parallel_profile_bytes,
        bytes.production_forward_additional_bytes,
        bytes.production_inverse_additional_bytes,
        bytes.production_bidirectional_additional_bytes,
    );
}

fn require_cap_refusal(
    length: usize,
    layout: Layout,
    catalog: &FftCatalog,
    cap: usize,
) -> Result<(), String> {
    match ParallelBatch::new(layout, catalog, cap - 1) {
        Err(error) if error == "resource-limit" => Ok(()),
        Err(_) => Err(format!("n={length} one-byte-short cap did not refuse")),
        Ok(_) => Err(format!("n={length} one-byte-short cap admitted")),
    }
}

fn warm_and_check_allocations(
    length: usize,
    serial: &mut SerialBatch,
    parallel: &mut ParallelBatch,
) -> Result<(), String> {
    serial.reset(100 + length);
    parallel.reset(100 + length)?;
    serial.execute(Operation::Forward)?;
    serial.execute(Operation::Inverse)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    let region = Region::new(GLOBAL);
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    let steady = region.change();
    if (
        steady.allocations,
        steady.deallocations,
        steady.reallocations,
    ) == (0, 0, 0)
    {
        Ok(())
    } else {
        Err(format!("n={length} steady allocation: {steady:?}"))
    }
}

fn interleaved_timings(
    length: usize,
    serial: &mut SerialBatch,
    parallel: &mut ParallelBatch,
) -> Result<Timings, String> {
    let mut timings = Timings::new();
    for pair in 0..3 {
        let values = if pair % 2 == 0 {
            (timed_serial_cycle(serial)?, timed_parallel_cycle(parallel)?)
        } else {
            let parallel_times = timed_parallel_cycle(parallel)?;
            (timed_serial_cycle(serial)?, parallel_times)
        };
        timings.record(pair, values.0, values.1);
        timings.emit_pair(length, pair);
    }
    Ok(timings)
}

fn validate_outputs(
    length: usize,
    serial: &mut SerialBatch,
    parallel: &mut ParallelBatch,
) -> Result<String, String> {
    serial.reset(700 + length);
    parallel.reset(700 + length)?;
    validate_pair(serial, parallel)?;
    let first_hash = parallel.hashes()?;
    validate_repeat(length, parallel, &first_hash)?;
    Ok(first_hash[0].0.clone())
}

fn validate_pair(serial: &mut SerialBatch, parallel: &mut ParallelBatch) -> Result<(), String> {
    serial.execute(Operation::Forward)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    controls::compare(serial, parallel, Operation::Forward)?;
    serial.execute(Operation::Inverse)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    controls::compare(serial, parallel, Operation::Inverse)?;
    Ok(())
}

fn validate_repeat(
    length: usize,
    parallel: &mut ParallelBatch,
    first_hash: &[(String, String)],
) -> Result<(), String> {
    parallel.reset(700 + length)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    if first_hash != parallel.hashes()?.as_slice() {
        return Err(format!("n={length} repeat hashes changed"));
    }
    Ok(())
}

fn timed_serial_cycle(batch: &mut SerialBatch) -> Result<(Duration, Duration), String> {
    let started = Instant::now();
    batch.execute(Operation::Forward)?;
    let forward = started.elapsed();
    let started = Instant::now();
    batch.execute(Operation::Inverse)?;
    Ok((forward, started.elapsed()))
}

fn timed_parallel_cycle(batch: &mut ParallelBatch) -> Result<(Duration, Duration), String> {
    let started = Instant::now();
    batch.execute(Operation::Forward).map_err(worker_debug)?;
    let forward = started.elapsed();
    let started = Instant::now();
    batch.execute(Operation::Inverse).map_err(worker_debug)?;
    Ok((forward, started.elapsed()))
}

fn timed_dispatch(batch: &mut ParallelBatch) -> Result<Duration, String> {
    let repeats = 101;
    let started = Instant::now();
    for _ in 0..repeats {
        batch.execute(Operation::Noop).map_err(worker_debug)?;
    }
    Ok(started.elapsed().div_f64(repeats as f64))
}

fn timed_copy(
    batch: &mut ParallelBatch,
    operation: Operation,
    repeats: usize,
) -> Result<Duration, String> {
    let started = Instant::now();
    for _ in 0..repeats {
        batch.copy_published(operation)?;
    }
    Ok(started.elapsed().div_f64(repeats as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_profile_covers_timing_and_publication_paths() {
        let backend = FftBackend::RustFft6_4_1AvxFma;
        if backend.ensure_available().is_err() {
            return;
        }
        let bytes = FftCatalog::reservation(backend).unwrap();
        let catalog = FftCatalog::new(backend, bytes).unwrap();
        controls::validate(&catalog).unwrap();
        profile(6, &catalog).unwrap();
    }
}
