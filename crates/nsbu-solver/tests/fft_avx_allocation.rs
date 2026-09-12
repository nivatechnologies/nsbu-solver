//! Dedicated explicit-AVX FFT process: steady counters exclude planner construction.
use nsbu_solver::{
    domain::Layout,
    spectral::{FftBackend, FftPlan},
    Complex64,
};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[cfg(target_arch = "x86_64")]
fn has_required_avx() -> bool {
    std::is_x86_feature_detected!("avx")
        && std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("fma")
}

#[cfg(not(target_arch = "x86_64"))]
fn has_required_avx() -> bool {
    false
}

fn main() {
    if !has_required_avx() {
        return;
    }
    let layout = Layout::new([96; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let reservation = FftPlan::reservation_with_backend(layout, backend).unwrap();
    let (plan, mut work) = FftPlan::new_with_backend(layout, backend, reservation).unwrap();
    let input = (0..layout.real_len())
        .map(|index| ((19 * index + 5) % 101) as f64 / 101.0)
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut output = vec![0.0; layout.real_len()];
    plan.forward(&input, &mut spectrum, &mut work).unwrap();
    plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    let region = Region::new(GLOBAL);
    for _ in 0..4 {
        plan.forward(&input, &mut spectrum, &mut work).unwrap();
        plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    }
    assert!(input
        .iter()
        .zip(&output)
        .all(|(left, right)| (left - right).abs() < 3e-13));
    no_allocations(region.change());
}

fn no_allocations(stats: Stats) {
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.deallocations, 0);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.bytes_deallocated, 0);
}
