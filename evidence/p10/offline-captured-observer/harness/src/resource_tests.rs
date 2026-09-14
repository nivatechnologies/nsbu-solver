//! Observer resource admission and forged-ledger regression checks.
use super::*;

#[test]
fn exact_resource_bound_refuses_cap_minus_one() {
    let fixture = matched_fixture("bound");
    let roomy = parse(execute(
        Mode::Preflight,
        &fixture.manifest_path,
        16,
        1,
        1 << 40,
        "owned-radix",
    ));
    let total = roomy["ledger"]["total_bytes"]
        .as_u64()
        .expect("ledger total");
    assert_eq!(roomy["fits"], true);
    let tight = parse(execute(
        Mode::Preflight,
        &fixture.manifest_path,
        16,
        1,
        usize::try_from(total - 1).expect("cap"),
        "owned-radix",
    ));
    assert_eq!(tight["fits"], false);
    let refusal = execute(
        Mode::Run,
        &fixture.manifest_path,
        16,
        1,
        usize::try_from(total - 1).expect("cap"),
        "owned-radix",
    )
    .expect_err("cap-minus-one must refuse");
    assert!(refusal.contains("resource preflight refusal"), "{refusal}");
    execute(
        Mode::Run,
        &fixture.manifest_path,
        16,
        1,
        usize::try_from(total).expect("cap"),
        "owned-radix",
    )
    .expect("exact-cap run allocates");
}


#[test]
fn n512_ledger_is_arithmetic_only_and_reports_the_copied_record_profile() {
    let record = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("clock1536-record.json");
    let value = parse(crate::n512_ledger(&record, 1 << 31));
    assert_eq!(value["schema"], "p10-offline-captured-observer-v1");
    assert_eq!(value["mode"], "n512-ledger");
    assert_eq!(value["qualification"], false);
    assert_eq!(value["profile"]["dimensions"][0], 512);
    assert_eq!(value["profile"]["workers"], 32);
    assert_eq!(value["profile"]["backend"], "rustfft-6.4.1-avx-avx2-fma");
    let copied: Value = serde_json::from_slice(&std::fs::read(&record).expect("copied record"))
        .expect("record json");
    let identity_len = copied["identity"].as_str().expect("identity").len();
    assert_eq!(
        value["snapshot_identity_bytes"].as_u64(),
        Some(identity_len as u64)
    );
    let total = value["budget"]["total_bytes"]
        .as_u64()
        .expect("total bytes");
    assert!(total > 0);
    assert_eq!(
        value["budget"]["baccus_512_gib_nominal_bytes"].as_u64(),
        Some(549_755_813_888)
    );
    assert_eq!(value["fits"], false, "N512 must not fit a 2 GiB cap");
    let tight = parse(crate::n512_ledger(
        &record,
        usize::try_from(total - 1).expect("cap"),
    ));
    assert_eq!(tight["fits"], false);
    let roomy = parse(crate::n512_ledger(
        &record,
        usize::try_from(total).expect("cap"),
    ));
    assert_eq!(roomy["fits"], true);
    assert_eq!(roomy["budget"]["total_bytes"].as_u64(), Some(total));
}


#[test]
fn forged_or_mismatched_ledgers_are_refused_before_allocation() {
    let domain = Domain::new(N, [1.0; 3], 1.0).expect("domain");
    let samples = Layout::new([32; 3]).expect("observer samples");
    let backend = FftBackend::OwnedRadix;
    let catalog = nsbu_solver::spectral::FftCatalog::new(backend, 0).expect("catalog");
    let ledger = OfflineObserver::preflight(domain, samples, 2, backend, 0).unwrap();

    // A forged ledger claiming half the reservation would bypass any cap at or above
    // that fabricated total; it must be refused by re-derivation, not by the cap check.
    let mut half_total = ledger;
    half_total.total_bytes /= 2;
    let mut shrunken_force = ledger;
    shrunken_force.total_bytes = shrunken_force
        .total_bytes
        .checked_sub(shrunken_force.force_storage_bytes)
        .unwrap()
        + std::mem::size_of::<ParallelReducedV2Force>();
    shrunken_force.force_storage_bytes = std::mem::size_of::<ParallelReducedV2Force>();
    let mut forged_backend = ledger;
    forged_backend.backend = FftBackend::RustFft6_4_1AvxFma;
    let mut forged_identity = ledger;
    forged_identity.snapshot_state_bytes -= 1;
    forged_identity.total_bytes -= 1;
    // Genuine ledgers from a different profile (samples layout / worker count).
    let cross_samples =
        OfflineObserver::preflight(domain, Layout::new([16; 3]).unwrap(), 2, backend, 0).unwrap();
    let cross_workers = OfflineObserver::preflight(domain, samples, 1, backend, 0).unwrap();

    for (name, forged, cap, expected) in [
        (
            "half-total",
            half_total,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "shrunken-force",
            shrunken_force,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "forged-backend",
            forged_backend,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "forged-identity",
            forged_identity,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "cross-samples",
            cross_samples,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "cross-workers",
            cross_workers,
            usize::MAX,
            SolverError::InvalidPayload,
        ),
        (
            "cap-minus-one",
            ledger,
            ledger.total_bytes - 1,
            SolverError::ResourceLimit,
        ),
    ] {
        assert_eq!(
            OfflineObserver::new(domain, samples, 2, &catalog, 0, &forged, cap).err(),
            Some(expected),
            "{name} ledger must be refused"
        );
    }
    OfflineObserver::new(domain, samples, 2, &catalog, 0, &ledger, ledger.total_bytes)
        .expect("genuine ledger at exact cap still constructs");
}
