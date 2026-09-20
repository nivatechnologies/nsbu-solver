use super::*;
use super::attempt::atomic_file_with;
#[cfg(not(feature = "n384-prep"))]
use nsbu_solver::{
    diagnostics::balances::BalanceSample,
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    SolverError,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn atomic_file_refuses_overwrite_and_leaves_no_partial_file() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("p10-endpoint-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).unwrap();
    publish_status(&root, "status.json", "first").unwrap();
    assert_eq!(
        publish_status(&root, "status.json", "second")
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::read_to_string(root.join("status.json")).unwrap(),
        "first"
    );
    assert!(!root.join("status.json.partial").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_write_publishes_nothing_and_removes_partial_file() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("p10-endpoint-fail-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let result = atomic_file_with(&root, "node.json", |path| {
        write_file(path, b"incomplete")?;
        Err(io::Error::other("injected after write"))
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
    assert!(!root.join("node.json").exists());
    assert!(!root.join("node.json.partial").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(not(feature = "n384-prep"))]
#[test]
fn disk_preflight_refuses_more_than_the_fixed_cap() {
    assert_eq!(
        disk_preflight(DISK_CAP_BYTES, 2),
        Err(SolverError::ResourceLimit)
    );
}

#[test]
fn json_string_escapes_quotes_newlines_and_control_characters() {
    assert_eq!(
        json_string("quoted \"line\"\nslash\\tab\t\u{1}"),
        "\"quoted \\\"line\\\"\\nslash\\\\tab\\t\\u0001\""
    );
}

#[cfg(not(feature = "n384-prep"))]
#[test]
fn scheduled_node_stages_and_publishes_snapshot_record_and_attempt_together() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "p10-endpoint-bundle-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 0,
            overhead: 0,
        },
        1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let state =
        SpectralState::from_rest(plan, TickClock::from_rest(-20, 8192).unwrap(), Epoch(0))
            .unwrap();
    let staged = stage_node(
        &root,
        &state,
        NodeRecord {
            identity: "test \"identity\"\nline",
            balance: BalanceSample::REST,
            observer_seconds: 0.0,
            force_seconds: 0.0,
            conservative_seconds: 0.0,
            transfer_measure_seconds: 0.0,
        },
        Some("{\"outcome\":\"committed\"}\n"),
    )
    .unwrap();
    let partial = root.join("node-0000.partial");
    assert!(partial.join("state.bin").exists());
    assert!(partial.join("record.json").exists());
    assert!(partial.join("attempt.json").exists());
    assert!(!root.join("node-0000").exists());
    staged.publish().unwrap();
    let published = root.join("node-0000");
    assert!(published.join("state.bin").exists());
    assert!(published.join("record.json").exists());
    assert!(published.join("attempt.json").exists());
    let record = fs::read_to_string(published.join("record.json")).unwrap();
    assert!(record.contains("\"identity\": \"test \\\"identity\\\"\\nline\""));
    assert!(!partial.exists());
    fs::remove_dir_all(root).unwrap();
}

/// Writer-side truth source for the launch packet: the tiny h32 snapshot
/// fixture and its layout/identity documents are produced by the REAL
/// `write_snapshot` writer using the REAL staged production profile identity
/// (`config::identity()` with the frozen `RUN_SOURCE`), never by hand.
#[cfg(feature = "n512-m512-temporal-h32")]
#[test]
fn real_writer_emits_the_minimal_h32_snapshot_fixture() {
    use crate::config;
    use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
    use sha2::{Digest, Sha256};
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let resources = ResourcePlan::new(
        domain,
        ExtraStorage { fft: 0, force: 0, diagnostics: 0, overhead: 0 },
        1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let state =
        SpectralState::from_rest(resources, TickClock::from_rest(-20, 8192).unwrap(), Epoch(0))
            .unwrap();
    let identity = config::identity();
    let fields: Vec<(&str, &str)> = identity
        .split(';')
        .map(|part| {
            let mut halves = part.splitn(2, '=');
            let key = halves.next().unwrap_or_default();
            let value = halves.next().unwrap_or_default();
            assert!(!key.is_empty() && !value.is_empty() && halves.next().is_none(),
                    "production identity part is not a single key=value field: {part:?}");
            (key, value)
        })
        .collect();
    assert!(fields.iter().any(|(key, _)| *key == "source"));
    assert!(identity.contains("profile=n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995"));
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("p10-h32-fixture-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let (hash, coefficient_bytes) =
        write_snapshot(&root.join("state.bin"), &state, &identity).unwrap();
    let bytes = fs::read(root.join("state.bin")).unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(coefficient_bytes, 48 * 3 * 16);
    assert_eq!(&bytes[0..12], b"P10AVXSNAP1\0");
    let identity_len = usize::try_from(u64::from_le_bytes(bytes[12..20].try_into().unwrap())).unwrap();
    assert_eq!(identity_len, identity.len());
    assert_eq!(&bytes[20..20 + identity_len], identity.as_bytes());
    let words: Vec<u128> = (0..4)
        .map(|index| {
            let start = 20 + identity_len + index * 16;
            u128::from_le_bytes(bytes[start..start + 16].try_into().unwrap())
        })
        .collect();
    assert_eq!(words, vec![0, 8192, 0, 0]);
    let payload = 20 + identity_len + 64;
    assert_eq!(
        &bytes[payload + coefficient_bytes..],
        Sha256::digest(&bytes[payload..payload + coefficient_bytes]).as_slice()
    );
    assert_eq!(hash, format!("{:x}", Sha256::digest(&bytes[payload..payload + coefficient_bytes])));
    assert_eq!(bytes.len(), payload + coefficient_bytes + 32);
    if let Some(target) = std::env::var_os("NSBU_SNAPSHOT_FIXTURE_OUT") {
        write_fixture_documents(&bytes, &identity, &fields, &PathBuf::from(target));
    }
}

#[cfg(feature = "n512-m512-temporal-h32")]
fn write_fixture_documents(bytes: &[u8], identity: &str, fields: &[(&str, &str)], target: &Path) {
    use sha2::{Digest, Sha256};
    if target.exists() {
        fs::remove_file(target).unwrap();
    }
    fs::write(target, bytes).unwrap();
    let identity_len = identity.len();
    let payload = 20 + identity_len + 64;
    let coefficient_bytes = bytes.len() - payload - 32;
    let field_json = fields
        .iter()
        .map(|(key, value)| format!("[{}, {}]", json_string(key), json_string(value)))
        .collect::<Vec<_>>()
        .join(",\n      ");
    let mut cursor = 0usize;
    let mut layout_fields = Vec::new();
    let push = |name: &str, length: usize, meaning: &str, cursor: &mut usize| {
        let entry = format!(
            "    {{\"offset\": {cursor}, \"length\": {length}, \"name\": {}, \"meaning\": {}}}",
            json_string(name),
            json_string(meaning),
        );
        *cursor += length;
        entry
    };
    layout_fields.push(push("magic", 12, "literal bytes 'P10AVXSNAP1' followed by one NUL byte (writer: write_all(b\"P10AVXSNAP1\\0\"))", &mut cursor));
    layout_fields.push(push("identity_len", 8, "u64 little-endian byte length of the identity text that follows (writer: (identity.len() as u64).to_le_bytes())", &mut cursor));
    layout_fields.push(push("identity", identity_len, "the exact staged production profile identity: config::identity() compiled with the frozen RUN_SOURCE; no padding or terminator", &mut cursor));
    for (name, meaning) in [
        ("clock_elapsed", "u128 little-endian state.clock().elapsed(); fixture value 0"),
        ("clock_target", "u128 little-endian state.clock().target(); fixture value 8192"),
        ("epoch", "u128 little-endian state.epoch().0; fixture value 0"),
        ("accepted_steps", "u128 little-endian state.accepted_steps(); fixture value 0"),
    ] {
        layout_fields.push(push(name, 16, meaning, &mut cursor));
    }
    layout_fields.push(push("coefficient_payload", coefficient_bytes, "three axes written in order axis 0,1,2; per axis half_len=48 complex coefficients in storage order; each coefficient is u64 f64-to-bits of value.re immediately followed by u64 f64-to-bits of value.im (16 bytes per coefficient, 768 bytes per axis, 2304 bytes total for this fixture). Offsets scale with the retained grid: length = half_len * 3 axes * 16.", &mut cursor));
    layout_fields.push(push("coefficient_sha256", 32, "trailing 32-byte SHA-256 digest computed over ONLY the coefficient_payload bytes; not over the header, identity, clock fields or the digest itself. This is the value published as record.json state_sha256 in hex.", &mut cursor));
    assert_eq!(cursor, bytes.len());
    let with_offset = layout_fields.join(",\n");
    let layout = format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-n512-h32-snapshot-layout-v1\",\n",
            "  \"fixture_file\": \"n512-h32-snapshot-minimal.bin\",\n",
            "  \"fixture_sha256\": {},\n  \"fixture_bytes\": {},\n",
            "  \"derived_from_writer\": \"artifact/snapshot.rs fn write_snapshot (state.bin writer; same writer used by step bundles)\",\n",
            "  \"generator_test\": \"artifact::tests::real_writer_emits_the_minimal_h32_snapshot_fixture (cargo test --features n512-m512-temporal-h32, env NSBU_SNAPSHOT_FIXTURE_OUT / NSBU_SNAPSHOT_LAYOUT_OUT / NSBU_IDENTITY_DOC_OUT)\",\n",
            "  \"fixture_state\": {{\n    \"grid\": \"4x4x3 physical (smallest grid Domain::new accepts: multiples of four, minimum 4)\",\n",
            "    \"half_spectrum_coefficients_per_axis\": 48,\n",
            "    \"clock_fields\": {{\"elapsed_ticks\": 0, \"target_ticks\": 8192, \"epoch\": 0, \"accepted_steps\": 0}},\n",
            "    \"note\": \"SpectralState::from_rest with TickClock::from_rest(-20, 8192), Epoch(0); identity is the exact staged production profile identity.\"\n  }},\n",
            "  \"endian\": \"all integer and float-bit fields little-endian\",\n",
            "  \"identity_text\": {},\n  \"identity_fields\": [\n      {}\n  ],\n",
            "  \"fields\": [\n{}\n  ],\n",
            "  \"total_bytes_formula\": \"12 + 8 + identity_len + 64 + half_len * 3 * 16 + 32\"\n}}\n"
        ),
        json_string(&format!("{:x}", Sha256::digest(bytes))),
        bytes.len(),
        json_string(identity),
        field_json,
        with_offset,
    );
    if let Some(path) = std::env::var_os("NSBU_SNAPSHOT_LAYOUT_OUT") {
        fs::write(PathBuf::from(path), layout).unwrap();
    }
    if let Some(path) = std::env::var_os("NSBU_IDENTITY_DOC_OUT") {
        let doc_text = format!(
            concat!(
                "{{\n  \"schema\": \"p10-avx-n512-m512-h32-production-identity-v1\",\n",
                "  \"identity_text\": {},\n  \"identity_len\": {},\n",
                "  \"fields\": [\n      {}\n  ]\n}}\n"
            ),
            json_string(identity),
            identity_len,
            field_json,
        );
        fs::write(PathBuf::from(path), doc_text).unwrap();
    }
}
