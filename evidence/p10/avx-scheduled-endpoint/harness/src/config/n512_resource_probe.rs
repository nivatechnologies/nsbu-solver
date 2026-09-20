use super::*;
use crate::schedule;
use nsbu_solver::SolverError;

#[test]
fn report_each_exact_api_reservation_without_allocation() {
    assert!(identity()
        .contains("external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"));
    assert!(identity().contains("rhs_fft_helpers=8;rhs_fft_total_workers=11"));
    assert!(identity().contains("provider_fft_helpers=8;provider_fft_total_workers=11"));
    let geometry = geometry().unwrap();
    let (catalog, force, rhs) = execution_reservations(geometry).unwrap();
    assert_eq!(
        (catalog, force.storage_bytes, rhs),
        (29_362_480, 29_180_171_864, 91_860_200_792)
    );
    assert_eq!(
        diagnostic_reservations(geometry).unwrap(),
        (30_182_212_728, 0)
    );
    let observer = crate::observer::ReducedObserver::preflight(
        geometry.domain,
        geometry.observer_samples,
        WORKERS,
        geometry.backend,
    )
    .unwrap();
    assert_eq!(observer, 232_283_988_248);
    assert_eq!(
        nsbu_solver::spectral::W3FftPool::additional_parallel_reservation_with_backend(
            geometry.domain.padded_layout().unwrap(),
            geometry.backend,
            nsbu_solver::spectral::W3FftMode::Bidirectional,
            FFT_WORKERS,
        )
        .unwrap(),
        21_812_652_048,
    );
    assert_eq!(
        nsbu_solver::spectral::W3FftPool::additional_parallel_reservation_with_backend(
            geometry.samples,
            geometry.backend,
            nsbu_solver::spectral::W3FftMode::Forward,
            FFT_WORKERS,
        )
        .unwrap(),
        4_343_035_792,
    );
    let reservations = reservations(geometry).unwrap();
    assert_eq!(
        resources(geometry.domain, reservations, CAP)
            .unwrap()
            .total(),
        CAP
    );
    assert_eq!(
        resources(geometry.domain, reservations, CAP - 1),
        Err(SolverError::ResourceLimit)
    );
    let combined = Reservations {
        observer,
        ..reservations
    };
    assert_eq!(
        resources(geometry.domain, combined, usize::MAX)
            .unwrap()
            .total(),
        439_911_439_400
    );
    assert_eq!(schedule::validate(), Ok(()));
    assert_eq!(admit().unwrap().resources.total(), CAP);
    assert_eq!(disk_bound(geometry.domain), Ok(155_226_537_984));
    assert_eq!(observer_work(geometry), Ok(1_108_101_562_368));
}

#[test]
fn run_gate_is_exact_and_preflight_remains_available() {
    assert_eq!(require_n512_gate(None), Err(SolverError::InvalidPayload));
    assert_eq!(
        require_n512_gate(Some("0")),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(require_n512_gate(Some("1")), Ok(()));
    assert_eq!(preflight().unwrap().total(), CAP);
}
