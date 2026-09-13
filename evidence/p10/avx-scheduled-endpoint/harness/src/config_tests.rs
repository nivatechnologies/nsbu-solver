use super::*;

#[cfg(feature = "n384-h32")]
const EXPECTED_N384_TOTAL: usize = 185_783_161_608;
#[cfg(feature = "n384-h64")]
const EXPECTED_N384_TOTAL: usize = 185_782_899_464;
#[cfg(all(
    feature = "n384-piecewise-common",
    not(feature = "n192-piecewise-cadv33"),
    not(feature = "n256-piecewise-cadv33"),
    not(feature = "n384-piecewise-ho-cadv33")
))]
const EXPECTED_N384_TOTAL: usize = 185_782_833_928;
#[cfg(feature = "n384-piecewise-ho-cadv33")]
const EXPECTED_N384_TOTAL: usize = 198_987_813_712;
#[cfg(feature = "n192-piecewise-cadv33")]
const EXPECTED_N384_TOTAL: usize = 52_428_314_504;
#[cfg(feature = "n256-piecewise-cadv33")]
const EXPECTED_N384_TOTAL: usize = 79_485_917_960;
#[cfg(feature = "n192-piecewise-cadv33")]
const EXPECTED_MATCHED_DISK: usize = 8_242_397_184;
#[cfg(feature = "n256-piecewise-cadv33")]
const EXPECTED_MATCHED_DISK: usize = 19_482_083_328;
#[cfg(feature = "n192-piecewise-cadv33")]
const EXPECTED_MATCHED_WORK: usize = 4_213_502_118_720;
#[cfg(feature = "n256-piecewise-cadv33")]
const EXPECTED_MATCHED_WORK: usize = 4_221_931_883_328;

#[test]
fn selected_profile_is_exact_and_execution_ready() {
    #[cfg(not(any(feature = "n384-matched-piecewise")))]
    assert_eq!(N, 384);
    #[cfg(feature = "n192-piecewise-cadv33")]
    assert_eq!(N, 192);
    #[cfg(feature = "n256-piecewise-cadv33")]
    assert_eq!(N, 256);
    assert_eq!(M, 384);
    #[cfg(feature = "n384-piecewise-ho-cadv33")]
    assert_eq!(method(), Method::HochbruckOstermann);
    #[cfg(not(feature = "n384-piecewise-common"))]
    assert_eq!(ADVECTIVE_LIMIT, 0.8);
    #[cfg(feature = "n384-piecewise")]
    assert_eq!(ADVECTIVE_LIMIT, 1.6);
    #[cfg(feature = "n384-cadv33-profile")]
    assert_eq!(ADVECTIVE_LIMIT, 3.3);
    assert_eq!(CAP, 206_158_430_208);
    assert_eq!(require_execution_ready(), Ok(()));
    let identity = identity();
    assert!(identity.contains("provider=parallel-reduced-v2-force-w3-attempt-cache"));
    assert!(identity.contains(&format!("method={METHOD_NAME}")));
    assert!(identity.contains(&format!("rhs_fft={RHS_FFT_IDENTITY}")));
    assert!(identity.contains("force_w3=layout384-width3-forward-add1827942144"));
    assert!(identity.contains("observer_force_samples=768"));
    assert!(identity.contains(&format!("observer_conservative={}", 2 * N)));
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
    #[cfg(feature = "n384-matched-piecewise")]
    {
        assert_eq!(admission.disk, EXPECTED_MATCHED_DISK);
        assert_eq!(admission.integration_work, EXPECTED_MATCHED_WORK);
        assert_eq!(admission.observer_work, 467_480_346_624);
    }
    assert!(matches!(
        admit_geometry(geometry, total - 1),
        Err(SolverError::ResourceLimit)
    ));
}
