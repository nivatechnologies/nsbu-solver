//! Dedicated process: allocation counters exclude test-harness and concurrent-test activity.
use nsbu_solver::domain::Domain;
use nsbu_solver::spectral::RotationalWorkspace;
use nsbu_solver::Complex64;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let reservation = RotationalWorkspace::reservation(domain).unwrap();
    let planning = Region::new(GLOBAL);
    let mut work = RotationalWorkspace::new(domain, reservation).unwrap();
    let planned = planning.change();
    assert!(planned.bytes_allocated <= reservation);
    assert_eq!(planned.reallocations, 0);
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let [a, b, c] = &mut output;
        work.evaluate([&zero; 3], [&zero; 3], [a, b, c], &mut pressure)
            .unwrap();
    }
    let stats = region.change();
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.deallocations, 0);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.bytes_deallocated, 0);
}
