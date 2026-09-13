//! Shared-pool allocation control with three persistent external callers.
use nsbu_solver::{
    domain::Layout,
    spectral::{FftBackend, FftCatalog, FftPlan, ParallelFftExecutor},
    Complex64,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, sync::Barrier};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    for workers in [8, 16] {
        shared_callers(backend, workers);
    }
}

fn shared_callers(backend: FftBackend, workers: usize) {
    let layout = Layout::new([6, 96, 192]).unwrap();
    let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
    let cap = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let executor = ParallelFftExecutor::new(
        layout,
        backend,
        workers,
        ParallelFftExecutor::additional_reservation(layout, backend, workers).unwrap(),
    )
    .unwrap();
    let ready = Barrier::new(4);
    let done = Barrier::new(4);
    let release = Barrier::new(4);
    let mut observed = None;
    std::thread::scope(|scope| {
        for caller in 0..3 {
            let (catalog, executor, ready, done, release) =
                (&catalog, &executor, &ready, &done, &release);
            scope.spawn(move || {
                let (plan, mut work) = FftPlan::new_from_catalog(layout, catalog, cap).unwrap();
                let input: Vec<_> = (0..layout.real_len())
                    .map(|i| ((i * 37 + caller * 11) % 251) as f64 / 251.0)
                    .collect();
                let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
                let mut restored = vec![0.0; layout.real_len()];
                let mut reference = spectrum.clone();
                let mut inverse_reference = restored.clone();
                plan.forward(&input, &mut reference, &mut work).unwrap();
                plan.inverse(&reference, &mut inverse_reference, &mut work)
                    .unwrap();
                for _ in 0..2 {
                    ready.wait();
                    for _ in 0..4 {
                        executor
                            .forward(&plan, &input, &mut spectrum, &mut work)
                            .unwrap();
                        executor
                            .inverse(&plan, &spectrum, &mut restored, &mut work)
                            .unwrap();
                        assert!(spectrum
                            .iter()
                            .zip(&reference)
                            .all(|(a, b)| a.re.to_bits() == b.re.to_bits()
                                && a.im.to_bits() == b.im.to_bits()));
                        assert!(restored
                            .iter()
                            .zip(&inverse_reference)
                            .all(|(a, b)| a.to_bits() == b.to_bits()));
                    }
                    done.wait();
                }
                release.wait();
            });
        }
        ready.wait();
        done.wait();
        let region = Region::new(GLOBAL);
        ready.wait();
        done.wait();
        observed = Some(region.change());
        drop(region);
        release.wait();
    });
    let stats = observed.unwrap();
    assert_eq!(stats.allocations, 0, "workers={workers}, {stats:?}");
    assert_eq!(stats.reallocations, 0, "workers={workers}, {stats:?}");
    assert_eq!(stats.deallocations, 0, "workers={workers}, {stats:?}");
    assert!(!executor.is_terminated());
    println!("workers={workers} callers=3 measured_roundtrips=12 exact_words=true allocations=0");
}
