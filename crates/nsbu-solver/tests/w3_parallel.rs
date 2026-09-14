//! Shared-pool W3 lifetime, exact-word, and long-run allocation guard.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, W3FftMode, W3FftPool};
use nsbu_solver::Complex64;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type RealField = [Vec<f64>; 3];
type SpectralField = [Vec<Complex64>; 3];

fn fixture() -> Option<(FftBackend, Layout, FftCatalog, usize)> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().ok()?;
    let layout = Layout::new([6; 3]).unwrap();
    let catalog_cap = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_cap).unwrap();
    let lane_cap = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    Some((backend, layout, catalog, lane_cap))
}

fn pool(layout: Layout, catalog: &FftCatalog, lane_cap: usize, helpers: usize) -> W3FftPool {
    let (seed_plan, seed_work) = FftPlan::new_from_catalog(layout, catalog, lane_cap).unwrap();
    let seed_spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let cap = W3FftPool::additional_parallel_reservation_with_backend(
        layout,
        catalog.backend(),
        W3FftMode::Bidirectional,
        helpers,
    )
    .unwrap();
    W3FftPool::from_scalar_lane_parallel(
        layout,
        catalog,
        W3FftMode::Bidirectional,
        helpers,
        (seed_plan, seed_work, seed_spectrum),
        cap,
    )
    .unwrap()
}

fn assert_identity(pool: &W3FftPool, helpers: usize) {
    let identity = pool.parallel_fft_identity().unwrap();
    assert_eq!(identity.helper_workers, helpers);
    assert_eq!(identity.persistent_callers, 3);
    assert_eq!(identity.workers, helpers + 3);
}

fn first_roundtrip(
    pool: &mut W3FftPool,
    layout: Layout,
) -> (RealField, RealField, SpectralField, RealField) {
    let mut inputs: RealField = std::array::from_fn(|lane| {
        (0..layout.real_len())
            .map(|index| ((17 * index + 11 * lane + 3) % 101) as f64 / 101.0 - 0.3)
            .collect()
    });
    let original = inputs.clone();
    pool.forward3(&mut inputs).unwrap();
    assert_eq!(inputs, original);
    let mut spectra: SpectralField =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    for (lane, spectrum) in spectra.iter_mut().enumerate() {
        pool.with_spectrum(lane, |actual| spectrum.copy_from_slice(actual))
            .unwrap();
        pool.prepare_inverse(lane, |target| target.copy_from_slice(spectrum))
            .unwrap();
    }
    let mut outputs: RealField = std::array::from_fn(|_| vec![0.0; layout.real_len()]);
    pool.inverse3(&mut outputs).unwrap();
    (inputs, original, spectra, outputs)
}

fn assert_serial_oracle(
    layout: Layout,
    catalog: &FftCatalog,
    lane_cap: usize,
    original: &RealField,
    spectra: &SpectralField,
    outputs: &RealField,
) {
    let (serial, mut serial_work) = FftPlan::new_from_catalog(layout, catalog, lane_cap).unwrap();
    for lane in 0..3 {
        let mut expected_spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        serial
            .forward(&original[lane], &mut expected_spectrum, &mut serial_work)
            .unwrap();
        assert!(spectra[lane]
            .iter()
            .zip(&expected_spectrum)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits()));
        let mut expected = vec![0.0; layout.real_len()];
        serial
            .inverse(&expected_spectrum, &mut expected, &mut serial_work)
            .unwrap();
        assert!(outputs[lane]
            .iter()
            .zip(expected)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
    }
}

fn assert_steady(pool: &mut W3FftPool, inputs: &mut RealField, outputs: &mut RealField) {
    for _ in 0..4 {
        pool.forward3(inputs).unwrap();
        pool.inverse3(outputs).unwrap();
    }
    let region = Region::new(GLOBAL);
    for _ in 0..96 {
        pool.forward3(inputs).unwrap();
        pool.inverse3(outputs).unwrap();
    }
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
fn three_persistent_callers_run_over_sixty_three_rounds_without_allocating() {
    let Some((_, layout, catalog, lane_cap)) = fixture() else {
        return;
    };
    let helpers = 2;
    let mut pool = pool(layout, &catalog, lane_cap, helpers);
    assert_identity(&pool, helpers);
    let (mut inputs, original, spectra, mut outputs) = first_roundtrip(&mut pool, layout);
    assert_serial_oracle(layout, &catalog, lane_cap, &original, &spectra, &outputs);
    assert_steady(&mut pool, &mut inputs, &mut outputs);
}
