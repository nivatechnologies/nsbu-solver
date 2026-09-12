//! Dedicated FFT process: allocation counters exclude test-harness activity.
use nsbu_solver::{domain::Layout, spectral::FftPlan, Complex64};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let layout = Layout::new([4, 6, 8]).unwrap();
    let reservation = FftPlan::reservation(layout).unwrap();
    let (plan, mut work) = FftPlan::new(layout, reservation).unwrap();
    let input = (0..layout.real_len())
        .map(|index| ((19 * index + 5) % 101) as f64 / 101.0)
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut output = vec![0.0; layout.real_len()];
    plan.forward(&input, &mut spectrum, &mut work).unwrap();
    plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        plan.forward(&input, &mut spectrum, &mut work).unwrap();
        plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    }
    assert!(input
        .iter()
        .zip(&output)
        .all(|(a, b)| (a - b).abs() < 3e-14));
    no_allocations(region.change());
}

fn no_allocations(stats: Stats) {
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.deallocations, 0);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.bytes_deallocated, 0);
}
