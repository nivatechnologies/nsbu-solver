use super::{FftBackend, FftCatalog, FftPlan, W3FftMode, W3FftPool, WIDTH};
use crate::{domain::Layout, Complex64, SolverError};

const BACKEND: FftBackend = FftBackend::RustFft6_4_1AvxFma;

#[cfg(target_arch = "x86_64")]
fn avx_available() -> bool {
    std::is_x86_feature_detected!("avx")
        && std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("fma")
}

#[cfg(not(target_arch = "x86_64"))]
fn avx_available() -> bool {
    false
}

fn catalog() -> FftCatalog {
    let cap = FftCatalog::reservation(BACKEND).unwrap();
    FftCatalog::new(BACKEND, cap).unwrap()
}

fn seed(layout: Layout, catalog: &FftCatalog) -> (FftPlan, super::FftWorkspace, Vec<Complex64>) {
    let cap = FftPlan::reservation_from_catalog(layout, catalog).unwrap();
    let (plan, workspace) = FftPlan::new_from_catalog(layout, catalog, cap).unwrap();
    let spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    (plan, workspace, spectrum)
}

fn pool(layout: Layout, catalog: &FftCatalog, mode: W3FftMode) -> W3FftPool {
    let cap = W3FftPool::additional_reservation(layout, catalog, mode).unwrap();
    W3FftPool::from_scalar_lane(layout, catalog, mode, seed(layout, catalog), cap).unwrap()
}

fn inputs(layout: Layout) -> [Vec<f64>; WIDTH] {
    std::array::from_fn(|lane| {
        (0..layout.real_len())
            .map(|index| ((17 * index + 13 * lane + 5) % 113) as f64 / 113.0 - 0.4)
            .collect()
    })
}

fn ownership(values: &[Vec<f64>; WIDTH]) -> [(*const f64, usize, usize); WIDTH] {
    std::array::from_fn(|lane| {
        (
            values[lane].as_ptr(),
            values[lane].len(),
            values[lane].capacity(),
        )
    })
}

fn same_bits(left: &[Complex64], right: &[Complex64]) -> bool {
    left.iter()
        .zip(right)
        .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}

fn failure_drains_every_lane(panic: bool) {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    for lane in 0..WIDTH {
        let mut owner = pool(layout, &catalog, W3FftMode::Forward);
        let mut values = inputs(layout);
        let prior = values.clone();
        let allocation = ownership(&values);
        owner.inject_failure(lane, panic);
        assert_eq!(
            owner.forward3(&mut values),
            Err(SolverError::ArithmeticResolutionLimited),
            "lane {lane}"
        );
        assert_eq!(values, prior, "lane {lane}");
        assert_eq!(ownership(&values), allocation, "lane {lane}");
        assert!(owner.all_collected(), "lane {lane}");
        assert!(owner.is_terminated(), "lane {lane}");
        for spectrum_lane in 0..WIDTH {
            assert_eq!(
                owner.with_spectrum(spectrum_lane, |_| ()),
                Err(SolverError::ArithmeticResolutionLimited),
                "failed lane {lane}, spectrum lane {spectrum_lane}"
            );
        }
        assert_eq!(
            owner.forward3(&mut values),
            Err(SolverError::ArithmeticResolutionLimited),
            "lane {lane}"
        );
    }
}

#[test]
fn every_injected_numerical_failure_drains_and_terminates_the_owner() {
    failure_drains_every_lane(false);
}

#[test]
fn every_caught_injected_panic_drains_and_terminates_the_owner() {
    failure_drains_every_lane(true);
}

#[test]
fn exact_additional_cap_constructs_and_one_byte_short_refuses() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    for mode in [W3FftMode::Forward, W3FftMode::Bidirectional] {
        let cap = W3FftPool::additional_reservation(layout, &catalog, mode).unwrap();
        let owner =
            W3FftPool::from_scalar_lane(layout, &catalog, mode, seed(layout, &catalog), cap)
                .unwrap();
        assert_eq!(owner.identity().additional_bytes, cap);
        drop(owner);
        assert!(matches!(
            W3FftPool::from_scalar_lane(layout, &catalog, mode, seed(layout, &catalog), cap - 1),
            Err(SolverError::ResourceLimit)
        ));
    }
}

#[test]
fn admission_is_closed_to_fixture_6_and_experiment_lengths_through_768() {
    for edge in [384, 512, 576, 768] {
        let layout = Layout::new([edge; 3]).unwrap();
        assert!(super::admission::additional(layout, BACKEND, W3FftMode::Forward).is_ok());
        assert!(super::admission::additional(layout, BACKEND, W3FftMode::Bidirectional).is_ok());
    }
    for edge in [288, 510, 514, 1024, 1152] {
        let excluded = Layout::new([edge; 3]).unwrap();
        assert_eq!(
            super::admission::additional(excluded, BACKEND, W3FftMode::Forward),
            Err(SolverError::InvalidPayload)
        );
    }
    let supported = Layout::new([384; 3]).unwrap();
    assert_eq!(
        super::admission::additional(supported, FftBackend::OwnedRadix, W3FftMode::Forward),
        Err(SolverError::InvalidPayload)
    );
}

#[test]
fn admitted_force_layouts_have_exact_w3_reservations() {
    let m512 = Layout::new([512; 3]).unwrap();
    assert_eq!(
        super::admission::additional(m512, BACKEND, W3FftMode::Forward).unwrap(),
        4_318_465_792
    );
    let m768 = Layout::new([768; 3]).unwrap();
    assert_eq!(
        super::admission::additional(m768, BACKEND, W3FftMode::Forward).unwrap(),
        14_540_099_328
    );
    assert_eq!(
        super::admission::additional(m768, BACKEND, W3FftMode::Bidirectional).unwrap(),
        21_787_856_768
    );
}

#[test]
fn mismatched_seed_backend_refuses_before_owner_construction() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    let owned_cap = FftPlan::reservation(layout).unwrap();
    let (plan, workspace) = FftPlan::new(layout, owned_cap).unwrap();
    let spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let cap = W3FftPool::additional_reservation(layout, &catalog, W3FftMode::Forward).unwrap();
    assert!(matches!(
        W3FftPool::from_scalar_lane(
            layout,
            &catalog,
            W3FftMode::Forward,
            (plan, workspace, spectrum),
            cap,
        ),
        Err(SolverError::InvalidPayload)
    ));
}

#[test]
fn small_avx_w3_forward_matches_serial_bits_and_preserves_inputs() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    let mut owner = pool(layout, &catalog, W3FftMode::Forward);
    let mut values = inputs(layout);
    let prior = values.clone();
    let allocation = ownership(&values);
    owner.forward3(&mut values).unwrap();
    assert_eq!(values, prior);
    assert_eq!(ownership(&values), allocation);

    let cap = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let (serial, mut workspace) = FftPlan::new_from_catalog(layout, &catalog, cap).unwrap();
    for (lane, values) in values.iter().enumerate() {
        let mut expected = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        serial
            .forward(values, &mut expected, &mut workspace)
            .unwrap();
        let actual = owner
            .with_spectrum(lane, |spectrum| spectrum.to_vec())
            .unwrap();
        assert!(same_bits(&actual, &expected), "lane {lane}");
    }
}

#[test]
fn explicit_parallel_w3_shares_identity_and_matches_serial_bits() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    let workers = 2;
    for excluded in [1, 62] {
        assert_eq!(
            W3FftPool::additional_parallel_reservation_with_backend(
                layout,
                BACKEND,
                W3FftMode::Forward,
                excluded,
            ),
            Err(SolverError::InvalidPayload)
        );
    }
    let cap = W3FftPool::additional_parallel_reservation_with_backend(
        layout,
        BACKEND,
        W3FftMode::Forward,
        workers,
    )
    .unwrap();
    assert!(matches!(
        W3FftPool::from_scalar_lane_parallel(
            layout,
            &catalog,
            W3FftMode::Forward,
            workers,
            seed(layout, &catalog),
            cap - 1,
        ),
        Err(SolverError::ResourceLimit)
    ));
    let mut owner = W3FftPool::from_scalar_lane_parallel(
        layout,
        &catalog,
        W3FftMode::Forward,
        workers,
        seed(layout, &catalog),
        cap,
    )
    .unwrap();
    let identity = owner.parallel_fft_identity().unwrap();
    assert_eq!(identity.layout, layout);
    assert_eq!(identity.backend, BACKEND);
    assert_eq!(identity.helper_workers, workers);
    assert_eq!(identity.persistent_callers, WIDTH);
    assert_eq!(identity.workers, workers + WIDTH);
    assert_eq!(owner.identity().additional_bytes, cap);

    let mut values = inputs(layout);
    owner.forward3(&mut values).unwrap();
    let serial_cap = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let (serial, mut workspace) = FftPlan::new_from_catalog(layout, &catalog, serial_cap).unwrap();
    for (lane, values) in values.iter().enumerate() {
        let mut expected = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        serial
            .forward(values, &mut expected, &mut workspace)
            .unwrap();
        let actual = owner
            .with_spectrum(lane, |spectrum| spectrum.to_vec())
            .unwrap();
        assert!(same_bits(&actual, &expected), "lane {lane}");
    }
}

#[test]
fn parallel_w3_drains_failures_and_panics_before_termination() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    let cap = W3FftPool::additional_parallel_reservation_with_backend(
        layout,
        BACKEND,
        W3FftMode::Forward,
        8,
    )
    .unwrap();
    for panic in [false, true] {
        let mut owner = W3FftPool::from_scalar_lane_parallel(
            layout,
            &catalog,
            W3FftMode::Forward,
            8,
            seed(layout, &catalog),
            cap,
        )
        .unwrap();
        let mut values = inputs(layout);
        let prior = values.clone();
        let pointers = ownership(&values);
        owner.inject_failure(1, panic);
        assert!(owner.forward3(&mut values).is_err());
        assert!(owner.is_terminated());
        assert!(owner.all_collected());
        assert_eq!(values, prior);
        assert_eq!(ownership(&values), pointers);
        assert!(owner.forward3(&mut values).is_err());
        assert!(owner.all_collected());
        drop(owner);
    }
}

#[test]
fn parallel_w3_drop_collects_poisoned_parked_worker() {
    if !avx_available() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let catalog = catalog();
    let cap = W3FftPool::additional_parallel_reservation_with_backend(
        layout,
        BACKEND,
        W3FftMode::Bidirectional,
        8,
    )
    .unwrap();
    let owner = W3FftPool::from_scalar_lane_parallel(
        layout,
        &catalog,
        W3FftMode::Bidirectional,
        8,
        seed(layout, &catalog),
        cap,
    )
    .unwrap();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = owner.workers[0].shared.state.lock().unwrap();
        panic!("independent parked-worker poison control");
    }));
    assert!(poisoned.is_err());
    drop(owner);
}
