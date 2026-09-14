//! Opt-in bounded parallel FFT correctness, admission, and allocation contracts.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, ParallelFftExecutor};
use nsbu_solver::{Complex64, SolverError};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn bits(values: &[Complex64]) -> Vec<(u64, u64)> {
    values
        .iter()
        .map(|value| (value.re.to_bits(), value.im.to_bits()))
        .collect()
}

fn input(layout: Layout) -> Vec<f64> {
    (0..layout.real_len())
        .map(|index| ((index * 29 + 7) % 251) as f64 / 251.0 - 0.25)
        .collect()
}

#[test]
fn bounded_parallel_executor_matches_words_and_has_zero_allocations_in_measured_call() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let layout = Layout::new([6, 96, 6]).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let lane_bytes = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let (serial, mut serial_work) =
        FftPlan::new_from_catalog(layout, &catalog, lane_bytes).unwrap();
    let (parallel, mut parallel_work) =
        FftPlan::new_from_catalog(layout, &catalog, lane_bytes).unwrap();
    let extra = ParallelFftExecutor::additional_reservation(layout, backend, 2).unwrap();
    let executor = ParallelFftExecutor::new(layout, backend, 2, extra).unwrap();
    let values = input(layout);
    let mut expected = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut actual = expected.clone();
    serial
        .forward(&values, &mut expected, &mut serial_work)
        .unwrap();
    executor
        .forward(&parallel, &values, &mut actual, &mut parallel_work)
        .unwrap();
    assert_eq!(bits(&expected), bits(&actual));
    let mut expected_real = vec![0.0; layout.real_len()];
    let mut actual_real = expected_real.clone();
    serial
        .inverse(&expected, &mut expected_real, &mut serial_work)
        .unwrap();
    executor
        .inverse(&parallel, &actual, &mut actual_real, &mut parallel_work)
        .unwrap();
    assert_eq!(
        expected_real
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        actual_real.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    let region = Region::new(GLOBAL);
    executor
        .forward(&parallel, &values, &mut actual, &mut parallel_work)
        .unwrap();
    let change = region.change();
    assert_eq!(
        (
            change.allocations,
            change.deallocations,
            change.reallocations
        ),
        (0, 0, 0)
    );
}

#[test]
fn admission_refuses_wrong_backend_worker_bounds_and_one_byte_under_before_allocation() {
    let layout = Layout::new([96; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    assert_eq!(
        ParallelFftExecutor::additional_reservation(layout, FftBackend::OwnedRadix, 8),
        Err(SolverError::InvalidPayload)
    );
    for workers in [1, 65] {
        assert_eq!(
            ParallelFftExecutor::additional_reservation(layout, backend, workers),
            Err(SolverError::InvalidPayload)
        );
    }
    let exact = ParallelFftExecutor::additional_reservation(layout, backend, 8).unwrap();
    let region = Region::new(GLOBAL);
    assert!(matches!(
        ParallelFftExecutor::new(layout, backend, 8, exact - 1),
        Err(SolverError::ResourceLimit)
    ));
    let change = region.change();
    assert_eq!(
        (
            change.allocations,
            change.deallocations,
            change.reallocations
        ),
        (0, 0, 0)
    );
}
