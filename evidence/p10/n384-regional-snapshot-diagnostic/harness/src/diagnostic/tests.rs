use super::*;

fn reviewed_manifest(body: &str) -> Manifest {
    serde_json::from_str(body).unwrap()
}

fn m384_manifest() -> Manifest {
    reviewed_manifest(include_str!(
        "../../../../external-reference-bridge/inputs/clock0512/snapshot.json"
    ))
}

fn m512_manifest() -> Manifest {
    reviewed_manifest(include_str!(
        "../../../../external-reference-bridge/inputs/clock0512-m512/snapshot.json"
    ))
}

#[test]
fn exact_cap_and_work_are_closed() {
    let storage = storage().unwrap();
    assert_eq!(storage.total, CAP_BYTES);
    assert_eq!(storage.packed_reference_cache, DIMENSION.pow(3) * 30 * 8);
    assert_eq!(storage.two_magnitude_arrays, DIMENSION.pow(3) * 2 * 8);
    assert_eq!(storage.cached_region_labels, DIMENSION.pow(3));
}

#[test]
fn both_independently_evolved_clock512_histories_are_admitted() {
    let m384 = m384_manifest();
    let m512 = m512_manifest();
    assert_eq!(
        validate_snapshot(&m384)
            .unwrap()
            .integration_force_dimension,
        384
    );
    assert_eq!(
        validate_snapshot(&m512)
            .unwrap()
            .integration_force_dimension,
        512
    );
}

#[test]
fn cross_profile_binding_is_refused() {
    let mut manifest = m512_manifest();
    manifest.profile.as_mut().unwrap().value = M384_PROFILE.into();
    assert!(validate_snapshot(&manifest).is_err());
}

#[test]
fn cross_history_hash_binding_is_refused() {
    let mut manifest = m512_manifest();
    manifest.coefficient_sha256 = M384_COEFFICIENT_SHA256.into();
    manifest.file_sha256 = M384_SNAPSHOT_FILE_SHA256.into();
    assert!(validate_snapshot(&manifest).is_err());
}

#[test]
fn ordered_component_schedule_is_complete() {
    let velocity = (0..3)
        .map(|index| Quantity::Velocity.entry(index))
        .collect::<Vec<_>>();
    let gradient = (0..9)
        .map(|index| Quantity::Gradient.entry(index))
        .collect::<Vec<_>>();
    let hessian = (0..27)
        .map(|index| Quantity::OrderedHessian.entry(index))
        .collect::<Vec<_>>();
    assert_eq!(velocity.len(), 3);
    assert_eq!(gradient.len(), 9);
    assert_eq!(hessian.len(), 27);
    let mut canonical_hessian = hessian.clone();
    canonical_hessian.sort_unstable();
    canonical_hessian.dedup();
    assert_eq!(canonical_hessian.len(), 18);
    for component in 0..3 {
        assert_eq!(
            hessian
                .iter()
                .filter(|(observed, _)| *observed == component)
                .count(),
            9
        );
    }
}

#[test]
fn execution_gate_requires_review_and_idle_signal() {
    if std::env::var_os("P10_ROOT_FULL_RUN_REVIEW").is_none()
        && std::env::var_os("P10_RESIDUAL_LOCALIZATION_IDLE").is_none()
    {
        assert!(launch_gate(&"0".repeat(64)).is_err());
    }
}

#[test]
fn address_space_limit_parser_requires_numeric_soft_and_hard_bytes() {
    let limits = policy::parse_address_space_limits(
        "Limit Soft Limit Hard Limit Units\nMax address space 137438953472 137438953472 bytes\n",
    )
    .unwrap();
    assert_eq!(limits, [137_438_953_472; 2]);
    assert!(policy::parse_address_space_limits(
        "Limit Soft Limit Hard Limit Units\nMax address space unlimited unlimited bytes\n",
    )
    .is_err());
}

#[test]
fn cached_labels_partition_every_observation_once() {
    let errors = [1.0, 2.0, 3.0, 4.0, 5.0];
    let references = [5.0, 4.0, 3.0, 2.0, 1.0];
    let labels = [0, 1, 2, 3, 4];
    let (global, regions) = measurement::reduce::<3>(&errors, &references, &labels, 1e-8).unwrap();
    assert!(matches!(global, SampleOutput::Measured { samples: 5, .. }));
    for region in regions {
        assert!(matches!(
            region.sampled,
            SampleOutput::Measured { samples: 1, .. }
        ));
    }
    assert_eq!(regions[3].region, "collar");
}
