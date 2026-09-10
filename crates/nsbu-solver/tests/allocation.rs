//! Dedicated process: allocation counters exclude test-harness and concurrent-test activity.
use nsbu_solver::diagnostics::conservative::ConservativeWorkspace;
use nsbu_solver::diagnostics::sampling::SamplingWorkspace;
use nsbu_solver::domain::{Domain, Layout};
use nsbu_solver::spectral::RotationalWorkspace;
use nsbu_solver::Complex64;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    rotational();
    conservative();
    sampling();
}

fn rotational() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let reservation = RotationalWorkspace::reservation(domain).unwrap();
    let mut work = planned(reservation, || {
        RotationalWorkspace::new(domain, reservation).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let [a, b, c] = &mut output;
        work.evaluate([&zero; 3], [&zero; 3], [a, b, c], &mut pressure)
            .unwrap();
    }
    no_allocations(region.change());
}

fn conservative() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let reservation = ConservativeWorkspace::reservation(domain).unwrap();
    let mut work = planned(reservation, || {
        ConservativeWorkspace::new(domain, reservation).unwrap()
    });
    let velocity = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let force = vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()];
    let mut output = [force.clone(), force.clone(), force.clone()];
    let mut pressure = force.clone();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let [a, b, c] = &mut output;
        work.evaluate([&velocity; 3], [&force; 3], [a, b, c], &mut pressure)
            .unwrap();
    }
    no_allocations(region.change());
}

fn no_allocations(stats: stats_alloc::Stats) {
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.deallocations, 0);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.bytes_deallocated, 0);
}

fn sampling() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let reservation = SamplingWorkspace::reservation(domain, layout).unwrap();
    let mut work = planned(reservation, || {
        SamplingWorkspace::new(domain, layout, reservation).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let samples = work.sample([&zero; 3]).unwrap();
        assert_eq!(samples.velocity_maximum.value, 0.0);
        assert_eq!(samples.vorticity_maximum.value, 0.0);
    }
    no_allocations(region.change());
}

fn planned<T>(reservation: usize, allocate: impl FnOnce() -> T) -> T {
    let region = Region::new(GLOBAL);
    let value = allocate();
    let stats = region.change();
    assert!(stats.bytes_allocated <= reservation);
    assert_eq!(stats.reallocations, 0);
    value
}
