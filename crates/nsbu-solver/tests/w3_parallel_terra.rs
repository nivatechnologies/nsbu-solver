//! Shared-pool W3 lifetime, exact-word, and long-run allocation guard.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, W3FftMode, W3FftPool};
use nsbu_solver::Complex64;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn rectangular_helpers_eight_and_sixteen_are_exact_and_steady() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog_cap = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_cap).unwrap();
    let lane_cap = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    for helpers in [8, 16] {
        let (seed_plan, seed_work) = FftPlan::new_from_catalog(layout, &catalog, lane_cap).unwrap();
        let seed_spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        let cap = W3FftPool::additional_parallel_reservation_with_backend(
            layout,
            backend,
            W3FftMode::Bidirectional,
            helpers,
        )
        .unwrap();
        let mut pool = W3FftPool::from_scalar_lane_parallel(
            layout,
            &catalog,
            W3FftMode::Bidirectional,
            helpers,
            (seed_plan, seed_work, seed_spectrum),
            cap,
        )
        .unwrap();
        let identity = pool.parallel_fft_identity().unwrap();
        assert_eq!(identity.helper_workers, helpers);
        assert_eq!(identity.persistent_callers, 3);
        assert_eq!(identity.workers, helpers + 3);

        let mut inputs: [Vec<f64>; 3] = std::array::from_fn(|lane| {
            (0..layout.real_len())
                .map(|index| ((17 * index + 11 * lane + 3) % 101) as f64 / 101.0 - 0.3)
                .collect()
        });
        let original = inputs.clone();
        pool.forward3(&mut inputs).unwrap();
        assert_eq!(inputs, original);

        let mut spectra: [Vec<Complex64>; 3] =
            std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
        for (lane, spectrum) in spectra.iter_mut().enumerate() {
            pool.with_spectrum(lane, |actual| spectrum.copy_from_slice(actual))
                .unwrap();
            pool.prepare_inverse(lane, |target| target.copy_from_slice(spectrum))
                .unwrap();
        }
        let mut outputs: [Vec<f64>; 3] = std::array::from_fn(|_| vec![0.0; layout.real_len()]);
        pool.inverse3(&mut outputs).unwrap();

        let (serial, mut serial_work) =
            FftPlan::new_from_catalog(layout, &catalog, lane_cap).unwrap();
        for lane in 0..3 {
            let mut expected_spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
            serial
                .forward(&original[lane], &mut expected_spectrum, &mut serial_work)
                .unwrap();
            assert!(
                spectra[lane]
                    .iter()
                    .zip(&expected_spectrum)
                    .all(|(a, b)| a.re.to_bits() == b.re.to_bits()
                        && a.im.to_bits() == b.im.to_bits())
            );
            let mut expected = vec![0.0; layout.real_len()];
            serial
                .inverse(&expected_spectrum, &mut expected, &mut serial_work)
                .unwrap();
            assert!(outputs[lane]
                .iter()
                .zip(expected)
                .all(|(a, b)| a.to_bits() == b.to_bits()));
        }

        for _ in 0..4 {
            pool.forward3(&mut inputs).unwrap();
            pool.inverse3(&mut outputs).unwrap();
        }
        let region = Region::new(GLOBAL);
        for _ in 0..96 {
            pool.forward3(&mut inputs).unwrap();
            pool.inverse3(&mut outputs).unwrap();
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
}
