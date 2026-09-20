use super::*;
use crate::schedule;
use nsbu_solver::SolverError;

const EXPECTED_TOTAL: usize = 37_402_593_776;

#[test]
fn profile_is_exact_n256_and_distinct_from_n384_and_n512() {
    assert_eq!(N, 256);
    assert_eq!(M, 512);
    assert_eq!(OBSERVER_M, 512);
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(CAP, 68_719_476_736);
    assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
    assert_eq!(tolerances().relative, [1e-5; 2]);
    assert_eq!(schedule::MAXIMUM_ATTEMPTS, 48);
    assert_eq!(schedule::ENDPOINT, 4096);
    assert_eq!(require_execution_ready(), Ok(()));
    let identity = identity();
    assert!(identity.contains("profile=n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c"));
    assert!(identity.contains("retained=256"));
    assert!(identity.contains("force_samples=512"));
    assert!(identity.contains("rhs_dealias=384"));
    assert!(identity.contains("rhs_w3=layout384-width3-bidirectional-add2734010240"));
    assert!(identity.contains("force_w3=layout512-width3-forward-add4318465792"));
    assert!(identity.contains("observer_execution=offline-baccus-required"));
    assert!(identity.contains("schema=p10-avx-n256-m512-observer-state-v1"));
    assert!(identity.contains("provider=parallel-reduced-v2-force-w3-attempt-cache"));
    assert!(identity.contains("rhs_w3_workers=3"));
    assert!(identity.contains("host=baccus"));
    assert!(identity
        .contains("external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"));
    assert!(identity.contains("observer_force_samples=512"));
    assert!(identity.contains("observer_conservative=512"));
    assert!(!identity.contains("host=sulaco"));
    assert!(!identity.contains("pgid-watchdog-v2"));
    assert!(!identity.contains("n512"));
    assert!(!identity.contains("n384"));
    assert!(!identity.contains("parallel8"));
}

#[test]
fn scalar_w3_capture_reservations_are_exact_without_numerical_arrays() {
    let geometry = geometry().unwrap();
    let (catalog, force, rhs) = execution_reservations(geometry).unwrap();
    assert_eq!((catalog, rhs), (29_362_480, 22_866_181_176));
    assert_eq!(
        force.storage_bytes,
        15_015_554_504,
        "M512 scalar W3 force storage"
    );
    assert_eq!(
        diagnostic_reservations(geometry).unwrap(),
        (3_787_457_656, 0),
        "offline capture reserves no inline observer"
    );
    let reservations = reservations(geometry).unwrap();
    let plan = resources(geometry.domain, reservations, CAP).unwrap();
    let admission = work_admission(geometry, reservations, plan).unwrap();
    assert_eq!(admission.resources.total(), EXPECTED_TOTAL);
    assert_eq!(admission.integration_work, 9_987_522_825_024);
    assert_eq!(admission.observer_work, 138_512_695_296);
    assert_eq!(disk_bound(geometry.domain), Ok(19_482_083_328));
    assert_eq!(
        resources(geometry.domain, reservations, EXPECTED_TOTAL - 1),
        Err(SolverError::ResourceLimit)
    );
    assert_eq!(
        resources(geometry.domain, reservations, CAP).unwrap().total(),
        EXPECTED_TOTAL
    );
}

#[test]
fn run_gate_is_opt_in_and_preflight_admits() {
    assert_eq!(require_n256_gate(None), Err(SolverError::InvalidPayload));
    assert_eq!(
        require_n256_gate(Some("0")),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(require_n256_gate(Some("1")), Ok(()));
    assert_eq!(preflight().unwrap().total(), EXPECTED_TOTAL);
    assert_eq!(admit().unwrap().resources.total(), EXPECTED_TOTAL);
}
