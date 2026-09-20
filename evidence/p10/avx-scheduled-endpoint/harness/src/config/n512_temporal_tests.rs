use super::*;
use crate::{artifact, schedule};
use nsbu_solver::SolverError;

#[cfg(feature = "n512-m512-temporal-h32")]
const EXPECTED: ExpectedProfile = ExpectedProfile {
    cap: 207_627_647_760,
    disk: 310_453_075_968,
    disk_cap: 512 * 1024 * 1024 * 1024,
    coarse_steps: 64,
    attempts: 96,
    profile: "n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995",
    schedule: "h32-clocks0-through2048-then-h64-through4096",
};
#[cfg(feature = "n512-m512-temporal-h16")]
const EXPECTED: ExpectedProfile = ExpectedProfile {
    cap: 207_628_040_976,
    disk: 620_906_151_936,
    disk_cap: 768 * 1024 * 1024 * 1024,
    coarse_steps: 32,
    attempts: 192,
    profile: "n512-m512-h16to2048-h32to4096-cadv33-w3-pfft1ed6995",
    schedule: "h16-clocks0-through2048-then-h32-through4096",
};

struct ExpectedProfile {
    cap: usize,
    disk: usize,
    disk_cap: usize,
    coarse_steps: usize,
    attempts: usize,
    profile: &'static str,
    schedule: &'static str,
}

#[test]
fn temporal_profile_is_exact_n512_and_shares_only_dt() {
    assert_eq!(N, 512);
    assert_eq!(M, 512);
    assert_eq!(OBSERVER_M, 1024);
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(FFT_WORKERS, 8);
    assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
    assert_eq!(tolerances().relative, [1e-5; 2]);
    assert_eq!(schedule::MAXIMUM_ATTEMPTS, EXPECTED.attempts);
    assert_eq!(schedule::ENDPOINT, 4096);
    assert_eq!(CAP, EXPECTED.cap);
    assert_eq!(artifact::DISK_CAP_BYTES, EXPECTED.disk_cap);
    assert_eq!(schedule::IDENTITY, EXPECTED.schedule);
    assert_eq!(require_execution_ready(), Ok(()));
    assert_eq!(disk_bound(domain().unwrap()), Ok(EXPECTED.disk));
}

#[test]
fn temporal_identity_is_distinct_and_host_independent() {
    let identity = identity();
    assert!(identity.contains(&format!("profile={}", EXPECTED.profile)));
    assert!(identity.contains(&format!("schedule={}", EXPECTED.schedule)));
    assert!(identity.contains(&format!("maximum_attempts={}", EXPECTED.attempts)));
    assert!(identity.contains("observer_force_samples=1024"));
    assert!(identity.contains("observer_conservative=1024"));
    assert!(identity.contains("observer_execution=offline-baccus-required"));
    assert!(identity.contains("host_provenance=explicit-reviewed-host-required"));
    assert!(identity.contains(&format!("schema={OBSERVER_STATE_SCHEMA}")));
    assert!(identity
        .contains("external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline"));
    assert!(!identity.contains("host=sulaco"));
    assert!(!identity.contains("host=baccus"));
    assert!(!identity.contains("n256"));
    assert!(!identity.contains("n384"));
    assert!(!identity.contains("h64to2048-h128to4096"));
}

#[test]
fn temporal_resource_cap_is_exact_and_refuses_one_byte_under() {
    let geometry = geometry().unwrap();
    let reservations = reservations(geometry).unwrap();
    assert_eq!(
        (
            reservations.catalog,
            reservations.force.storage_bytes,
            reservations.rhs
        ),
        (29_362_480, 29_180_171_864, 91_860_200_792)
    );
    assert_eq!(diagnostic_reservations(geometry).unwrap(), (30_182_212_728, 0));
    let total = resources(geometry.domain, reservations, CAP)
        .unwrap()
        .total();
    assert_eq!(total, EXPECTED.cap);
    assert_eq!(
        resources(geometry.domain, reservations, EXPECTED.cap - 1),
        Err(SolverError::ResourceLimit)
    );
    assert_eq!(schedule::validate(), Ok(()));
    assert_eq!(admit().unwrap().resources.total(), EXPECTED.cap);
    assert_eq!(preflight().unwrap().total(), EXPECTED.cap);
}

#[test]
fn temporal_work_and_history_bounds_scale_with_attempts() {
    let geometry = geometry().unwrap();
    let reservations = reservations(geometry).unwrap();
    let plan = resources(geometry.domain, reservations, CAP).unwrap();
    let admission = work_admission(geometry, reservations, plan).unwrap();
    let per_attempt = reservations.force.work_units * 12;
    assert_eq!(
        admission.integration_work,
        per_attempt * EXPECTED.attempts
    );
    assert_eq!(admission.observer_work, 1_108_101_562_368);
    assert_eq!(admission.disk, EXPECTED.disk);
    assert_eq!(HISTORY_BYTES, EXPECTED.attempts * 4096);
    assert_eq!(schedule::FINE.len(), 9);
    assert_eq!(schedule::MIDDLE, [0, 1024, 2048, 3072, 4096]);
    assert_eq!(schedule::COARSE, [0, 2048, 4096]);
    assert_eq!(schedule::ENDPOINT, 4096);
    assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
    assert_eq!(tolerances().relative, [1e-5; 2]);
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(schedule::step(0), Ok((EXPECTED.coarse_steps / 2) as u128));
    assert_eq!(schedule::step(2048), Ok(EXPECTED.coarse_steps as u128));
}

#[test]
fn run_gate_requires_opt_in_and_explicit_reviewed_host() {
    assert_eq!(
        require_temporal_gate(None, None),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        require_temporal_gate(Some("1"), None),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        require_temporal_gate(Some("0"), Some("baccus")),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        require_temporal_gate(Some("1"), Some("cray")),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(require_temporal_gate(Some("1"), Some("baccus")), Ok(()));
    assert_eq!(require_temporal_gate(Some("1"), Some("sulaco")), Ok(()));
}
