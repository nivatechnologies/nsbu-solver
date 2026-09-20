use super::*;
use crate::artifact;
use nsbu_solver::SolverError;

#[cfg(feature = "n384-h32")]
const EXPECTED_N384_TOTAL: usize = 185_783_726_856;
#[cfg(feature = "n384-h64")]
const EXPECTED_N384_TOTAL: usize = 185_783_464_712;
#[cfg(all(
    feature = "n384-piecewise-common",
    not(feature = "n384-m512-piecewise-cadv33")
))]
const EXPECTED_N384_TOTAL: usize = 185_783_399_176;
#[cfg(feature = "n384-m512-piecewise-cadv33")]
const EXPECTED_N384_TOTAL: usize = 193_243_685_640;

#[test]
fn selected_profile_is_exact_and_execution_ready() {
    assert_eq!(N, 384);
    #[cfg(not(feature = "n384-m512-piecewise-cadv33"))]
    assert_eq!(M, 384);
    #[cfg(feature = "n384-m512-piecewise-cadv33")]
    assert_eq!(M, 512);
    assert_eq!(OBSERVER_M, 768);
    #[cfg(not(feature = "n384-piecewise-common"))]
    assert_eq!(ADVECTIVE_LIMIT, 0.8);
    #[cfg(feature = "n384-piecewise")]
    assert_eq!(ADVECTIVE_LIMIT, 1.6);
    #[cfg(any(
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    ))]
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(CAP, 206_158_430_208);
    assert_eq!(require_execution_ready(), Ok(()));
    let identity = identity();
    assert!(identity.contains("provider=parallel-reduced-v2-force-w3-attempt-cache"));
    assert!(identity.contains("rhs_w3=layout576-width3-bidirectional-add9200926592"));
    assert!(identity.contains(&format!("force_w3={}", force_w3_identity())));
    assert!(identity.contains("observer_force_samples=768"));
    assert!(identity.contains("observer_conservative=768"));
    assert!(identity.contains("numa=whole-host-unbound-all-visible-cpus-memory"));
    assert!(identity.contains(&format!("external_stop={EXTERNAL_STOP}")));
    assert!(identity.contains(&format!("artifact_cap={}", artifact::DISK_CAP_BYTES)));
}

#[test]
fn complete_w3_reservation_is_available_without_execution_admission() {
    let geometry = geometry().unwrap();
    let admission = admit_geometry(geometry, CAP).unwrap();
    let total = admission.resources.total();
    println!(
        "n384_complete_reservation={total} classes={:?} catalog={} force_storage={} rhs={} attempt={} observer={} overhead={} disk={}",
        admission.resources.classes(),
        admission.reservations.catalog,
        admission.reservations.force.storage_bytes,
        admission.reservations.rhs,
        admission.reservations.attempt,
        admission.reservations.observer,
        OVERHEAD,
        admission.disk,
    );
    assert_eq!(total, EXPECTED_N384_TOTAL);
    #[cfg(feature = "n384-m512-piecewise-cadv33")]
    {
        assert_eq!(admission.reservations.force.storage_bytes, 19_816_721_864);
        assert_eq!(admission.reservations.rhs, 46_267_774_008);
        assert_eq!(admission.reservations.observer, 98_075_369_752);
        assert_eq!(admission.integration_work, 10_022_091_230_016);
        assert_eq!(admission.observer_work, 467_480_346_624);
        assert_eq!(admission.disk, 65_573_289_984);
        assert_eq!(CAP - total, 12_914_744_568);
        assert_eq!(tolerances().absolute, [1e-5, 1e-4]);
        assert_eq!(tolerances().relative, [1e-5; 2]);
    }
    assert!(matches!(
        admit_geometry(geometry, total - 1),
        Err(SolverError::ResourceLimit)
    ));
}
