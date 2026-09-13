use super::*;
use std::fs;

#[test]
fn exact_storage_plan_accounts_for_every_live_buffer() {
    let bridge = fixture_bridge([6; 3], [4; 3]);
    let snapshot = fixture_snapshot([4; 3]);
    let plan = preflight(&bridge, &snapshot).unwrap();
    assert_eq!(plan.storage.physical_reference_velocity, 6 * 6 * 6 * 3 * 8);
    assert_eq!(plan.storage.fft_output_spectrum, 6 * 6 * 4 * 16);
    assert_eq!(plan.storage.retained_reference_velocity, 4 * 4 * 3 * 3 * 16);
    assert_eq!(plan.work.reference_evaluations, 216);
    assert_eq!(plan.work.scalar_forward_transforms, 3);
}

#[test]
fn binding_rejects_shift_projection_case_and_clock_changes() {
    let mut snapshot = fixture_snapshot([4; 3]);
    let mut bridge = fixture_bridge([6; 3], [4; 3]);
    validate_binding(&bridge, &snapshot).unwrap();
    validate_snapshot_review(&bridge, &snapshot).unwrap();
    snapshot.profile.as_mut().unwrap().kind = crate::model::ProfileBindingKind::LegacyFullIdentity;
    assert!(validate_snapshot_review(&bridge, &snapshot).is_err());
    snapshot.profile.as_mut().unwrap().kind =
        crate::model::ProfileBindingKind::IdentityProfileField;
    snapshot.epoch += 1;
    assert!(validate_snapshot_review(&bridge, &snapshot).is_err());
    snapshot.epoch -= 1;
    snapshot.evolution.schedule[0].step_ticks = 128;
    assert!(validate_snapshot_review(&bridge, &snapshot).is_err());
    snapshot.evolution.schedule[0].step_ticks = 64;
    snapshot.identity = snapshot.identity.replace(
        &format!("execution_cap={SNAPSHOT_EXECUTION_CAP}"),
        "execution_cap=1",
    );
    assert!(validate_snapshot_review(&bridge, &snapshot).is_err());
    snapshot = fixture_snapshot([4; 3]);
    bridge.coordinate_shift = true;
    assert!(validate_bridge(&bridge).is_err());
    bridge.coordinate_shift = false;
    bridge.projection = "leray".into();
    assert!(validate_bridge(&bridge).is_err());
    bridge.projection = PROJECTION.into();
    bridge.case_sha256 = "0".repeat(64);
    assert!(validate_bridge(&bridge).is_err());
    bridge.case_sha256 = CASE_SHA256.into();
    bridge.elapsed += 1;
    assert!(validate_binding(&bridge, &snapshot).is_err());

    let mut endpoint = fixture_snapshot([4; 3]);
    endpoint.elapsed = 4096;
    endpoint.evolution.comparison_endpoint = 4096;
    endpoint.evolution.schedule[0].until_exclusive = 2048;
    endpoint
        .evolution
        .schedule
        .push(crate::model::ScheduleSegment {
            from_inclusive: 2048,
            until_exclusive: 4096,
            step_ticks: 128,
        });
    endpoint.epoch = 48;
    endpoint.accepted_steps = 48;
    let mut endpoint_bridge = fixture_bridge([6; 3], [4; 3]);
    endpoint_bridge.elapsed = 4096;
    validate_snapshot_review(&endpoint_bridge, &endpoint).unwrap();
    endpoint.elapsed = 300;
    assert!(validate_snapshot_review(&endpoint_bridge, &endpoint).is_err());
}

#[test]
fn tiny_sampled_reference_is_finite_and_keeps_raw_divergence() {
    let bridge = fixture_bridge([96, 6, 6], [4; 3]);
    let snapshot = fixture_snapshot([4; 3]);
    let plan = preflight(&bridge, &snapshot).unwrap();
    let reference = produce_reference(&bridge, &plan).unwrap();
    for component in &reference {
        nsbu_solver::domain::validate_spectrum(
            snapshot.domain().unwrap().layout(),
            component,
            1e-12,
        )
        .unwrap();
    }
    let norms = crate::absolute::measure(
        snapshot.domain().unwrap(),
        reference.each_ref().map(Vec::as_slice),
    )
    .unwrap();
    assert!(norms.l2.is_finite() && norms.l2 > 0.0);
    assert!(norms.h1 >= norms.l2);
    assert!(norms.divergence_l2.is_finite());
}

#[test]
fn snapshot_decoder_accepts_bound_bytes_and_rejects_malformed_framing() {
    let root = std::env::temp_dir().join(format!(
        "p10-external-reference-bridge-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let mut snapshot = fixture_snapshot([4; 3]);
    snapshot.snapshot = root.join("state.bin");
    let coefficients: [Vec<Complex64>; 3] = std::array::from_fn(|_| {
        vec![Complex64::new(0.0, 0.0); snapshot.domain().unwrap().layout().half_len()]
    });
    let bytes = encode_snapshot(&snapshot, &coefficients);
    let payload = 12 + 8 + snapshot.identity.len() + 4 * 16;
    snapshot.coefficient_sha256 =
        format!("{:x}", Sha256::digest(&bytes[payload..bytes.len() - 32]));
    snapshot.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&snapshot.snapshot, &bytes).unwrap();
    let decoded = crate::decode::load(&snapshot).unwrap();
    assert_eq!(decoded.clock.elapsed, 512);
    assert_eq!(decoded.coefficient_sha256, snapshot.coefficient_sha256);
    assert_eq!(decoded.file_sha256, snapshot.file_sha256);

    let bridge = fixture_bridge([6; 3], [4; 3]);
    let plan = preflight(&bridge, &snapshot).unwrap();
    let input = BoundInput {
        bridge,
        snapshot: snapshot.clone(),
        preflight: plan,
    };
    assert!(execute(&input, plan.storage.total - 1).is_err());
    let diagnostic = execute(&input, plan.storage.total).unwrap();
    assert_eq!(diagnostic.difference.l2, diagnostic.sampled_reference.l2);
    assert_eq!(diagnostic.actual.l2, 0.0);

    fs::write(&snapshot.snapshot, &bytes[..bytes.len() - 1]).unwrap();
    assert!(crate::decode::load(&snapshot)
        .unwrap_err()
        .contains("length mismatch"));
    let mut wrong_magic = bytes;
    wrong_magic[0] ^= 1;
    fs::write(&snapshot.snapshot, wrong_magic).unwrap();
    assert!(crate::decode::load(&snapshot)
        .unwrap_err()
        .contains("magic mismatch"));
}

#[test]
fn production_preflight_binds_sparse_length_without_decoding_state() {
    let root = std::env::temp_dir().join(format!(
        "p10-external-reference-bridge-{}-bind",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let plan_path = root.join("plan.json");
    fs::write(&plan_path, b"{}\n").unwrap();
    let mut snapshot = fixture_snapshot([384; 3]);
    snapshot.snapshot = root.join("state.bin");
    snapshot.plan = plan_path;
    snapshot.plan_sha256 = format!("{:x}", Sha256::digest(b"{}\n"));
    let state_bytes = crate::decode::state_bytes(&snapshot).unwrap();
    let file_bytes = 12 + 8 + snapshot.identity.len() + 4 * 16 + state_bytes + 32;
    fs::File::create(&snapshot.snapshot)
        .unwrap()
        .set_len(file_bytes as u64)
        .unwrap();
    let snapshot_path = root.join("snapshot.json");
    let snapshot_json = serde_json::to_vec_pretty(&snapshot).unwrap();
    fs::write(&snapshot_path, &snapshot_json).unwrap();

    let mut bridge = fixture_bridge([768; 3], [384; 3]);
    bridge.snapshot_manifest = snapshot_path;
    bridge.snapshot_manifest_sha256 = format!("{:x}", Sha256::digest(&snapshot_json));
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    for (source, path) in bridge.reference_sources.iter_mut().zip([
        "crates/nsbu-benchmarks/src/scalar.rs",
        "crates/nsbu-benchmarks/src/root.rs",
        "crates/nsbu-benchmarks/src/time.rs",
        "crates/nsbu-benchmarks/src/error.rs",
        "crates/nsbu-benchmarks/src/lib.rs",
        "crates/nsbu-solver/src/domain/clock.rs",
        "crates/nsbu-benchmarks/data/similarity-mms-v2.json",
    ]) {
        source.path = source_root.join(path);
    }
    bridge.execution_cap_bytes = preflight(&bridge, &snapshot).unwrap().storage.total;
    let bridge_path = root.join("bridge.json");
    fs::write(&bridge_path, serde_json::to_vec_pretty(&bridge).unwrap()).unwrap();
    let output = crate::run(&[bridge_path.into_os_string(), "20900924104".into()]).unwrap();
    assert!(output.contains("\"status\": \"preflight-only\""));
    assert!(output.contains("\"total\": 20900924104"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_n384_m768_preflight_is_allocation_free() {
    let bridge = fixture_bridge([768; 3], [384; 3]);
    let snapshot = fixture_snapshot([384; 3]);
    let plan = preflight(&bridge, &snapshot).unwrap();
    println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    assert!(plan.storage.total > 20_000_000_000);
    assert!(plan.storage.total < 22_000_000_000);
    assert_eq!(plan.work.reference_evaluations, 768 * 768 * 768);
}

fn encode_snapshot(snapshot: &SnapshotManifest, coefficients: &[Vec<Complex64>; 3]) -> Vec<u8> {
    let mut bytes = b"P10AVXSNAP1\0".to_vec();
    bytes.extend_from_slice(&(snapshot.identity.len() as u64).to_le_bytes());
    bytes.extend_from_slice(snapshot.identity.as_bytes());
    for value in [
        snapshot.elapsed,
        snapshot.target,
        snapshot.epoch,
        snapshot.accepted_steps,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let payload = bytes.len();
    for component in coefficients {
        for value in component {
            bytes.extend_from_slice(&value.re.to_bits().to_le_bytes());
            bytes.extend_from_slice(&value.im.to_bits().to_le_bytes());
        }
    }
    bytes.extend_from_slice(&Sha256::digest(&bytes[payload..]));
    bytes
}

fn fixture_bridge(samples: [usize; 3], retained: [usize; 3]) -> BridgeManifest {
    BridgeManifest {
        schema: "p10-external-reference-bridge-input-v1".into(),
        snapshot_manifest: "snapshot.json".into(),
        snapshot_manifest_sha256: "a".repeat(64),
        case_sha256: CASE_SHA256.into(),
        reference_source_commit: REFERENCE_SOURCE_COMMIT.into(),
        reference_sources: SOURCES
            .into_iter()
            .map(|(role, sha256)| SourceBinding {
                role: role.into(),
                path: format!("{role}.rs").into(),
                sha256: sha256.into(),
            })
            .collect(),
        harness_source_commit: "a".repeat(40),
        binary_sha256: current_executable_sha256(),
        reference_evaluator: EVALUATOR.into(),
        arithmetic: ARITHMETIC.into(),
        sample_dimensions: samples,
        retained_dimensions: retained,
        physical_grid: GRID.into(),
        fft_backend: "rustfft-6.4.1-avx-avx2-fma".into(),
        fft_normalization: NORMALIZATION.into(),
        crop: CROP.into(),
        nyquist: NYQUIST.into(),
        projection: PROJECTION.into(),
        coordinate_shift: false,
        mean_alignment: false,
        classification: CLASSIFICATION.into(),
        execution_context: EXECUTION_CONTEXT.into(),
        execution_cap_bytes: 1,
        clock_exponent: -20,
        clock_target: 8192,
        elapsed: 512,
    }
}

fn fixture_snapshot(dimensions: [usize; 3]) -> SnapshotManifest {
    let source_commit = "d".repeat(40);
    SnapshotManifest {
        schema: "p10-snapshot-comparison-input-v1".into(),
        comparison_kind: crate::model::ComparisonKind::MatchedSpatial,
        snapshot: "state.bin".into(),
        plan: "plan.json".into(),
        identity: format!(
            "source={source_commit};case={CASE_SHA256};profile={PROFILE};execution_cap={SNAPSHOT_EXECUTION_CAP};artifact_cap={SNAPSHOT_ARTIFACT_CAP}"
        ),
        source_commit,
        plan_sha256: "e".repeat(64),
        coefficient_sha256: "f".repeat(64),
        file_sha256: "1".repeat(64),
        backend: "fixture".into(),
        execution: "fixture".into(),
        dimensions,
        evolution: crate::model::Evolution {
            case_sha256: CASE_SHA256.into(),
            quantum_exponent: -20,
            clock_target: 8192,
            comparison_endpoint: 512,
            lengths: [1.0; 3],
            viscosity: 1.0,
            method: "cox-matthews".into(),
            integration_force_dimensions: [384; 3],
            schedule: vec![crate::model::ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 512,
                step_ticks: 64,
            }],
            absolute_tolerances: [1e-5, 1e-4],
            relative_tolerances: [1e-5; 2],
        },
        elapsed: 512,
        target: 8192,
        epoch: 8,
        accepted_steps: 8,
        profile: Some(crate::model::ProfileBinding {
            kind: crate::model::ProfileBindingKind::IdentityProfileField,
            value: PROFILE.into(),
        }),
        admission_guard: Some(crate::model::AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        }),
        arithmetic_control: None,
    }
}

fn current_executable_sha256() -> String {
    let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
    format!("{:x}", Sha256::digest(bytes))
}
