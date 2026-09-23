use super::*;
use nsbu_solver::Complex64;

fn fixture_pair(
    dir: &std::path::Path,
) -> (Manifest, Manifest, [Vec<Complex64>; 3], [Vec<Complex64>; 3]) {
    let mut left = small_manifest(dir, 4, "left");
    let mut right = small_manifest(dir, 8, "right");
    finish_small_manifest(&mut left);
    finish_small_manifest(&mut right);
    let mut left_fields = zero_fields(domain([4; 3]).layout());
    hermitian_mode(
        domain([4; 3]).layout(),
        &mut left_fields,
        [1, 0, 0],
        [
            Complex64::new(0.5, 0.25),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    let mut right_fields = zero_fields(domain([8; 3]).layout());
    hermitian_mode(
        domain([8; 3]).layout(),
        &mut right_fields,
        [1, 0, 0],
        [
            Complex64::new(0.5, 0.25),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    );
    write_fixture(&mut left, &left_fields);
    write_fixture(&mut right, &right_fields);
    (left, right, left_fields, right_fields)
}

fn write_manifest(dir: &std::path::Path, name: &str, manifest: &Manifest) -> PathBuf {
    let path = dir.join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec(manifest).unwrap()).unwrap();
    path
}

#[test]
fn writer_schema_roundtrip_loads_both_sizes() {
    let dir = root("decoder-roundtrip");
    let (left, right, _, _) = fixture_pair(&dir);
    let left = decode::read_manifest(&write_manifest(&dir, "left", &left)).unwrap();
    let right = decode::read_manifest(&write_manifest(&dir, "right", &right)).unwrap();
    let left_snapshot = decode::load(&left).unwrap();
    let right_snapshot = decode::load(&right).unwrap();
    assert_eq!(left_snapshot.clock.elapsed, 64);
    assert_eq!(right_snapshot.clock.elapsed, 64);
    assert_eq!(left_snapshot.file_sha256, left.file_sha256);
}

fn load_failure(dir: &std::path::Path, manifest: &Manifest, expected: &str) {
    let manifest = decode::read_manifest(&write_manifest(dir, "case", manifest));
    match manifest {
        Ok(manifest) => {
            let error = decode::load(&manifest).unwrap_err();
            assert!(error.contains(expected), "{error}");
        }
        Err(error) => assert!(error.contains(expected), "{error}"),
    }
}

#[test]
fn decoder_refuses_bad_magic() {
    let dir = root("decoder-magic");
    let (mut left, _, _, _) = fixture_pair(&dir);
    let mut bytes = fs::read(&left.snapshot).unwrap();
    bytes[0] = b'X';
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "magic mismatch");
}

#[test]
fn decoder_refuses_truncated_and_extended_files() {
    let dir = root("decoder-length");
    let (left, _, _, _) = fixture_pair(&dir);
    let bytes = fs::read(&left.snapshot).unwrap();
    fs::write(&left.snapshot, &bytes[..bytes.len() - 16]).unwrap();
    load_failure(&dir, &left, "snapshot length mismatch");

    let dir = root("decoder-extend");
    let (mut left, _, _, _) = fixture_pair(&dir);
    let mut bytes = fs::read(&left.snapshot).unwrap();
    bytes.push(0);
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "snapshot length mismatch");
}

#[test]
fn decoder_refuses_trailing_bytes_with_consistent_declared_length() {
    let dir = root("decoder-trailing");
    let (mut left, _, _, _) = fixture_pair(&dir);
    let mut bytes = fs::read(&left.snapshot).unwrap();
    bytes.truncate(bytes.len() - 32);
    bytes.push(0);
    bytes.extend_from_slice(&[0_u8; 31]);
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    let manifest = decode::read_manifest(&write_manifest(&dir, "case", &left)).unwrap();
    let error = decode::load(&manifest).unwrap_err();
    assert!(
        error.contains("trailer mismatch") || error.contains("trailing bytes"),
        "{error}"
    );
}

#[test]
fn decoder_refuses_flipped_trailer_byte() {
    let dir = root("decoder-trailer");
    let (mut left, _, _, _) = fixture_pair(&dir);
    let mut bytes = fs::read(&left.snapshot).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "trailer mismatch");
}

#[test]
fn decoder_refuses_big_endian_coefficients_against_the_reviewed_hashes() {
    let dir = root("decoder-endianness");
    let (mut left, _, fields, _) = fixture_pair(&dir);
    let mut bytes = encode(&left, &fields);
    let payload = 12 + 8 + left.identity.len() + 4 * 16;
    let coefficient_len = bytes.len() - payload - 32;
    let swapped: Vec<u8> = bytes[payload..payload + coefficient_len]
        .chunks_exact(8)
        .flat_map(|word| word.iter().rev().copied().collect::<Vec<u8>>())
        .collect();
    bytes.splice(payload..payload + coefficient_len, swapped);
    let trailer = Sha256::digest(&bytes[payload..payload + coefficient_len]).to_vec();
    bytes.splice(bytes.len() - 32.., trailer);
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    // The manifest still carries the reviewed little-endian hashes, so the
    // byte-swapped artifact is refused rather than silently misread.
    load_failure(&dir, &left, "hash does not match reviewed manifest");
}

#[test]
fn decoder_refuses_big_endian_clock_words() {
    let dir = root("decoder-clock-endianness");
    let (left, _, fields, _) = fixture_pair(&dir);
    let mut bytes = encode(&left, &fields);
    let clock_start = 12 + 8 + left.identity.len();
    for word in 0..4 {
        let start = clock_start + word * 16;
        bytes[start..start + 16].reverse();
    }
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "clock mismatch");
}

#[test]
fn decoder_refuses_big_endian_identity_length() {
    let dir = root("decoder-identity-endianness");
    let (left, _, fields, _) = fixture_pair(&dir);
    let mut bytes = encode(&left, &fields);
    let start = 12;
    let end = 20;
    bytes[start..end].reverse();
    fs::write(&left.snapshot, &bytes).unwrap();
    let manifest = decode::read_manifest(&write_manifest(&dir, "case", &left)).unwrap();
    let error = decode::load(&manifest).unwrap_err();
    assert!(
        error.contains("identity length mismatch")
            || error.contains("identity length out of bound"),
        "{error}"
    );
}

#[test]
fn decoder_refuses_identity_length_and_byte_drift() {
    // Same manifest with a differently sized identity than the written file.
    let dir = root("decoder-identity-length");
    let (mut left, _, _, _) = fixture_pair(&dir);
    left.identity = left.identity.replace("fixture-left;", "fixture-leftx;");
    load_failure(&dir, &left, "length mismatch");

    // Same identity length but a flipped identity byte in the file.
    let dir = root("decoder-identity-byte");
    let (mut left, _, _, _) = fixture_pair(&dir);
    let mut bytes = fs::read(&left.snapshot).unwrap();
    bytes[12 + 8] ^= 0x01;
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "identity mismatch");
}

#[test]
fn decoder_refuses_clock_word_drift() {
    let dir = root("decoder-clock");
    let (mut left, _, fields, _) = fixture_pair(&dir);
    let mut bytes = encode(&left, &fields);
    let clock_start = 12 + 8 + left.identity.len();
    bytes[clock_start] ^= 0x08;
    left.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&left.snapshot, &bytes).unwrap();
    load_failure(&dir, &left, "clock mismatch");
}

#[test]
fn manifest_envelope_refusals() {
    let dir = root("manifest-envelope");
    let (base, _, _, _) = fixture_pair(&dir);

    let mut schema = base.clone();
    schema.schema = "p10-snapshot-comparison-input-v1".into();
    let error = decode::read_manifest(&write_manifest(&dir, "schema", &schema)).unwrap_err();
    assert!(
        error.contains("invalid pair comparison manifest binding"),
        "{error}"
    );

    let mut value = serde_json::to_value(&base).unwrap();
    value["comparison_kind"] = serde_json::json!("MATCHED_M512_SPATIAL_DIAGNOSTIC");
    let path = dir.join("old-kind.json");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let error = decode::read_manifest(&path).unwrap_err();
    assert!(error.contains("variant"), "{error}");

    let mut value = serde_json::to_value(&base).unwrap();
    value["unexpected_field"] = serde_json::json!(1);
    let path = dir.join("unknown-field.json");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(decode::read_manifest(&path).is_err());

    fs::write(dir.join("oversized.json"), vec![b' '; 65 * 1024]).unwrap();
    let error = decode::read_manifest(&dir.join("oversized.json")).unwrap_err();
    assert!(error.contains("exceeds 64 KiB bound"), "{error}");

    let mut clock_drift = base.clone();
    clock_drift.target = 65;
    let error = decode::read_manifest(&write_manifest(&dir, "drift", &clock_drift)).unwrap_err();
    assert!(error.contains("clock envelope"), "{error}");

    let mut hex = base.clone();
    hex.source_commit = "z".repeat(40);
    let error = decode::read_manifest(&write_manifest(&dir, "hex", &hex)).unwrap_err();
    assert!(
        error.contains("invalid pair comparison manifest binding"),
        "{error}"
    );

    let mut short_hex = base.clone();
    short_hex.file_sha256 = "a".repeat(42);
    let error = decode::read_manifest(&write_manifest(&dir, "short-hex", &short_hex)).unwrap_err();
    assert!(
        error.contains("invalid pair comparison manifest binding"),
        "{error}"
    );

    let mut wrong_force = base.clone();
    wrong_force.evolution.integration_force_dimensions = [384; 3];
    let error = decode::read_manifest(&write_manifest(&dir, "force", &wrong_force)).unwrap_err();
    assert!(
        error.contains("invalid pair evolution semantics"),
        "{error}"
    );

    let mut wrong_method = base.clone();
    wrong_method.evolution.method = "hochbruck-ostermann".into();
    let error = decode::read_manifest(&write_manifest(&dir, "method", &wrong_method)).unwrap_err();
    assert!(
        error.contains("invalid pair evolution semantics"),
        "{error}"
    );

    let mut bad_profile = base.clone();
    bad_profile.identity = bad_profile
        .identity
        .replace("profile=fixture-left", "profile=other");
    let error = decode::read_manifest(&write_manifest(&dir, "profile", &bad_profile)).unwrap_err();
    assert!(error.contains("profile does not match"), "{error}");

    let mut plan_drift = base.clone();
    plan_drift.plan_sha256 = "e".repeat(64);
    let error = decode::read_manifest(&write_manifest(&dir, "plan", &plan_drift)).unwrap_err();
    assert!(error.contains("frozen plan SHA-256 mismatch"), "{error}");
}

#[test]
fn preflight_rejects_file_length_drift_before_any_load() {
    let dir = root("preflight-length");
    let (left, right, _, _) = fixture_pair(&dir);
    let left = decode::read_manifest(&write_manifest(&dir, "left", &left)).unwrap();
    let right = decode::read_manifest(&write_manifest(&dir, "right", &right)).unwrap();
    assert!(decode::preflight(&left, &right).is_ok());
    fs::write(&right.snapshot, b"truncated").unwrap();
    let error = decode::preflight(&left, &right).unwrap_err();
    assert!(error.contains("snapshot length mismatch"), "{error}");
}
