use super::support::{direct, has_required_avx, same_bits};
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan};
use nsbu_solver::{Complex64, SolverError};

#[test]
fn explicit_avx_backend_matches_direct_dft_and_repeats_bits() {
    if !has_required_avx() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let reservation = FftPlan::reservation_with_backend(layout, backend).unwrap();
    let (plan, mut work) = FftPlan::new_with_backend(layout, backend, reservation).unwrap();
    assert_eq!(plan.backend(), backend);
    let values = (0..layout.real_len())
        .map(|index| ((17 * index + 3) % 101) as f64 / 101.0 - 0.25)
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    plan.forward(&values, &mut spectrum, &mut work).unwrap();
    for mode in [[0, 0, 0], [1, 2, 1], [5, 5, 3], [2, 1, 0]] {
        let expected = direct(&values, layout.dimensions(), mode);
        let actual = spectrum[layout.index(mode).unwrap()];
        assert!((actual - expected).norm_sqr() < 1e-27);
    }
    let first = spectrum.clone();
    plan.forward(&values, &mut spectrum, &mut work).unwrap();
    assert!(same_bits(&first, &spectrum));
    let mut restored = vec![0.0; values.len()];
    plan.inverse(&spectrum, &mut restored, &mut work).unwrap();
    assert!(values
        .iter()
        .zip(restored)
        .all(|(left, right)| (left - right).abs() < 3e-14));
}

#[test]
fn explicit_avx_backend_has_closed_lengths_caps_and_workspace_identity() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    assert_eq!(
        FftPlan::reservation_with_backend(Layout::new([8; 3]).unwrap(), backend),
        Err(SolverError::InvalidDomain)
    );
    let layout = Layout::new([1536, 6, 6]).unwrap();
    let reservation = FftPlan::reservation_with_backend(layout, backend).unwrap();
    assert!(matches!(
        FftPlan::new_with_backend(layout, backend, reservation - 1),
        Err(SolverError::ResourceLimit)
    ));
    for length in [1024, 1152, 1536] {
        assert!(
            FftPlan::reservation_with_backend(Layout::new([length, 6, 6]).unwrap(), backend,)
                .is_ok()
        );
    }
    if has_required_avx() {
        let small = Layout::new([6; 3]).unwrap();
        let small_reservation = FftPlan::reservation_with_backend(small, backend).unwrap();
        let (plan, _) = FftPlan::new_with_backend(layout, backend, reservation).unwrap();
        let (_, mut wrong) = FftPlan::new_with_backend(small, backend, small_reservation).unwrap();
        let real = vec![0.0; layout.real_len()];
        let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        assert_eq!(
            plan.forward(&real, &mut spectrum, &mut wrong),
            Err(SolverError::InvalidPayload)
        );
    }
}

#[test]
fn same_layout_cross_backend_workspaces_are_checked_before_transform() {
    if !has_required_avx() {
        return;
    }
    let layout = Layout::new([6; 3]).unwrap();
    let owned_bytes = FftPlan::reservation(layout).unwrap();
    let avx_bytes =
        FftPlan::reservation_with_backend(layout, FftBackend::RustFft6_4_1AvxFma).unwrap();
    let (owned, mut owned_work) = FftPlan::new(layout, owned_bytes).unwrap();
    let (avx, mut avx_work) =
        FftPlan::new_with_backend(layout, FftBackend::RustFft6_4_1AvxFma, avx_bytes).unwrap();
    let input = (0..layout.real_len())
        .map(|index| index as f64 / layout.real_len() as f64)
        .collect::<Vec<_>>();
    let mut owned_with_avx_work = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut avx_with_owned_work = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    owned
        .forward(&input, &mut owned_with_avx_work, &mut avx_work)
        .unwrap();
    avx.forward(&input, &mut avx_with_owned_work, &mut owned_work)
        .unwrap();
    assert!(owned_with_avx_work.iter().all(|value| value.is_finite()));
    assert!(avx_with_owned_work.iter().all(|value| value.is_finite()));
}

#[test]
fn largest_admitted_avx_lengths_execute_forward_and_inverse() {
    if !has_required_avx() {
        return;
    }
    for length in [1024, 1152, 1536] {
        let layout = Layout::new([length, 6, 6]).unwrap();
        let backend = FftBackend::RustFft6_4_1AvxFma;
        let reservation = FftPlan::reservation_with_backend(layout, backend).unwrap();
        let (plan, mut work) = FftPlan::new_with_backend(layout, backend, reservation).unwrap();
        let input = (0..layout.real_len())
            .map(|index| ((index * 17 + 5) % 251) as f64 / 251.0 - 0.25)
            .collect::<Vec<_>>();
        let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
        let mut restored = vec![0.0; layout.real_len()];
        plan.forward(&input, &mut spectrum, &mut work).unwrap();
        plan.inverse(&spectrum, &mut restored, &mut work).unwrap();
        assert!(input
            .iter()
            .zip(restored)
            .all(|(left, right)| (left - right).abs() < 2e-12));
    }
}

#[test]
fn execution_catalog_is_eager_shared_and_separately_reserved() {
    if !has_required_avx() {
        return;
    }
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    assert!(catalog_bytes >= 28 * 1024 * 1024);
    assert!(matches!(
        FftCatalog::new(backend, catalog_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let workspace_bytes = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    assert!(workspace_bytes < FftPlan::reservation_with_backend(layout, backend).unwrap());
    let (plan, _) = FftPlan::new_from_catalog(layout, &catalog, workspace_bytes).unwrap();
    assert_eq!(plan.backend(), backend);
}
