use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, ParallelFftExecutor};
use nsbu_solver::Complex64;
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use std::time::Instant;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "usage: benchmark N WORKERS");
    let n = args[1].parse::<usize>().unwrap();
    let workers = args[2].parse::<usize>().unwrap();
    let layout = Layout::new([n; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
    let lane_bytes = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let (serial, mut serial_work) = FftPlan::new_from_catalog(layout, &catalog, lane_bytes).unwrap();
    let (parallel, mut parallel_work) = FftPlan::new_from_catalog(layout, &catalog, lane_bytes).unwrap();
    let extra = ParallelFftExecutor::additional_reservation(layout, backend, workers).unwrap();
    let executor = ParallelFftExecutor::new(layout, backend, workers, extra).unwrap();
    let input = (0..layout.real_len()).map(|index| {
        let bits = (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17);
        (bits >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64)) - 0.5
    }).collect::<Vec<_>>();
    let mut serial_spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut parallel_spectrum = serial_spectrum.clone();
    let mut serial_real = vec![0.0; layout.real_len()];
    let mut parallel_real = serial_real.clone();
    serial.forward(&input, &mut serial_spectrum, &mut serial_work).unwrap();
    executor.forward(&parallel, &input, &mut parallel_spectrum, &mut parallel_work).unwrap();
    serial.inverse(&serial_spectrum, &mut serial_real, &mut serial_work).unwrap();
    executor.inverse(&parallel, &parallel_spectrum, &mut parallel_real, &mut parallel_work).unwrap();
    assert_eq!(hash_complex(&serial_spectrum), hash_complex(&parallel_spectrum));
    assert_eq!(hash_real(&serial_real), hash_real(&parallel_real));
    let started = Instant::now();
    serial.forward(&input, &mut serial_spectrum, &mut serial_work).unwrap();
    let serial_forward = started.elapsed().as_nanos();
    let forward_region = Region::new(GLOBAL);
    let started = Instant::now();
    executor.forward(&parallel, &input, &mut parallel_spectrum, &mut parallel_work).unwrap();
    let parallel_forward = started.elapsed().as_nanos();
    let forward_allocations = forward_region.change();
    let started = Instant::now();
    serial.inverse(&serial_spectrum, &mut serial_real, &mut serial_work).unwrap();
    let serial_inverse = started.elapsed().as_nanos();
    let inverse_region = Region::new(GLOBAL);
    let started = Instant::now();
    executor.inverse(&parallel, &parallel_spectrum, &mut parallel_real, &mut parallel_work).unwrap();
    let parallel_inverse = started.elapsed().as_nanos();
    let inverse_allocations = inverse_region.change();
    println!("{{\"n\":{n},\"workers\":{workers},\"lane_bytes\":{lane_bytes},\"additional_bytes\":{extra},\"serial_forward_ns\":{serial_forward},\"parallel_forward_ns\":{parallel_forward},\"serial_inverse_ns\":{serial_inverse},\"parallel_inverse_ns\":{parallel_inverse},\"forward_allocations\":[{},{},{}],\"inverse_allocations\":[{},{},{}],\"forward_sha256\":\"{}\",\"inverse_sha256\":\"{}\"}}", forward_allocations.allocations, forward_allocations.deallocations, forward_allocations.reallocations, inverse_allocations.allocations, inverse_allocations.deallocations, inverse_allocations.reallocations, hash_complex(&parallel_spectrum), hash_real(&parallel_real));
}

fn hash_complex(values: &[Complex64]) -> String {
    let mut hash = Sha256::new();
    for value in values { hash.update(value.re.to_bits().to_le_bytes()); hash.update(value.im.to_bits().to_le_bytes()); }
    format!("{:x}", hash.finalize())
}
fn hash_real(values: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in values { hash.update(value.to_bits().to_le_bytes()); }
    format!("{:x}", hash.finalize())
}
