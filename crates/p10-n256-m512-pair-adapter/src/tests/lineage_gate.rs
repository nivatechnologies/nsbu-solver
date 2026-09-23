use super::*;
use std::path::Path;

/// Build a contract-valid coarse manifest bound to a closed-schedule intake.
/// The intake fixture already writes the real metadata-only rest record; a
/// complete capture additionally writes full-length writer-schema states.
fn staged(dir: &Path, complete: bool) -> (Manifest, LineageIntake) {
    let source = "a".repeat(40);
    let plan_path = dir.join("coarse-frozen-plan.json");
    fs::write(&plan_path, b"{\"synthetic\":true}\n").unwrap();
    let plan_sha256 = format!("{:x}", Sha256::digest(b"{\"synthetic\":true}\n"));
    let (intake_path, intake) = intake_fixture(dir, &source, &plan_sha256);
    let mut manifest = coarse_manifest(dir, &source);
    manifest.plan_sha256 = plan_sha256;
    bind_intake(&mut manifest, &intake_path);
    let endpoint = &intake.states[47];
    manifest.coefficient_sha256 = endpoint.coefficient_sha256.clone();
    manifest.file_sha256 = endpoint.file_sha256.clone();
    manifest.snapshot = endpoint.path.clone();
    if complete {
        for record in &intake.states {
            let steps = contract::steps_through(record.clock).unwrap();
            write_sparse_state(
                &record.path,
                &manifest.identity,
                ClockHeader {
                    elapsed: record.clock,
                    target: contract::CLOCK_TARGET,
                    epoch: steps,
                    accepted_steps: steps,
                },
            );
        }
    }
    (manifest, intake)
}

fn write_state_for(record: &StateRecord, identity: &str, clock: u128, epoch: u128) {
    write_sparse_state(
        record.path.as_path(),
        identity,
        ClockHeader {
            elapsed: clock,
            target: contract::CLOCK_TARGET,
            epoch,
            accepted_steps: epoch,
        },
    )
}

// ---- Finding 2: a self-attested intake is metadata, not completed lineage ----

#[test]
fn self_attested_complete_lineage_is_unverified_at_runtime() {
    let dir = root("self-attested");
    let (manifest, intake) = staged(&dir, true);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_UNVERIFIED), "{error}");
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let error = lineage::admit_standalone(&intake_path).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_UNVERIFIED), "{error}");
    assert_eq!(intake.states.len(), 48);
}

#[test]
fn runtime_admission_does_not_trust_self_attested_endpoint_hashes() {
    // The review's reproduce: an unrelated identity, arbitrary hex-shaped hashes
    // and a complete set of full-length sparse headers must not be admitted.
    let dir = root("forged-self-attested");
    let (manifest, _) = staged(&dir, true);
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let error = lineage::admit_standalone(&intake_path).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_UNVERIFIED), "{error}");
    assert!(!error.contains("admitted"), "{error}");
}

// ---- The authentication path itself is real (accepts genuine, refuses forged) ----

#[test]
fn anchor_authenticated_complete_lineage_admits() {
    let dir = root("anchor-admit");
    let (manifest, intake) = staged(&dir, true);
    let anchor = trusted_anchor(&intake);
    let admitted = lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).unwrap();
    assert_eq!(admitted.value.states.len(), 48);
    assert_eq!(admitted.endpoint_file_sha256, format!("{:064x}", 1047));
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    assert!(lineage::admit_standalone_with_anchor(&intake_path, Some(&anchor)).is_ok());
}

#[test]
fn anchor_rejects_a_changed_committed_step_hash() {
    let dir = root("anchor-step-hash");
    let (mut manifest, intake) = staged(&dir, true);
    let anchor = trusted_anchor(&intake); // pinned to the ORIGINAL step hashes
                                          // Tamper one non-endpoint state's intake-declared file hash and rebind so the
                                          // intake self-hash still matches; only the trusted receipt can catch it.
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let mut tampered = intake.clone();
    tampered.states[10].file_sha256 = "f".repeat(64);
    fs::write(&intake_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    manifest.lineage.as_mut().unwrap().intake_sha256 =
        format!("{:x}", Sha256::digest(fs::read(&intake_path).unwrap()));
    let error = lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).unwrap_err();
    assert!(error.contains("not authenticated"), "{error}");
    assert!(error.contains("clock=704"), "{error}");
}

#[test]
fn anchor_rejects_a_wrong_identity_length_receipt() {
    let dir = root("anchor-wrong-identity");
    let (manifest, intake) = staged(&dir, true);
    let mut anchor = trusted_anchor(&intake);
    anchor.identity = format!("{};drift", anchor.identity);
    let error = lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).unwrap_err();
    assert!(error.contains("identity does not match"), "{error}");
}

// ---- Finding 1: the actual writer rest schema (no binary payload) ----

#[test]
fn real_writer_rest_schema_probes_cleanly_under_anchor() {
    let dir = root("real-rest-schema");
    // intake_fixture already wrote the exact p10-avx-n384-rest-v1 record.
    let (manifest, intake) = staged(&dir, true);
    let anchor = trusted_anchor(&intake);
    assert!(lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).is_ok());
    // Crucially it is never rejected as a binary snapshot on "magic mismatch".
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(!error.contains("magic mismatch"), "{error}");
    assert!(error.starts_with(lineage::LINEAGE_UNVERIFIED), "{error}");
}

#[test]
fn binary_rest_file_is_refused_as_not_the_rest_schema() {
    let dir = root("binary-rest-schema");
    let (mut manifest, intake) = staged(&dir, true);
    // Overwrite the rest with binary bytes (the writer never emits these) and sync
    // every declared hash, so the only possible refusal is schema conformance.
    let binary = b"P10AVXSNAP1\0not-a-rest-record-binary-payload";
    fs::write(dir.join("rest"), binary).unwrap();
    let binary_hash = format!("{:x}", Sha256::digest(binary));
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let mut tampered = intake.clone();
    tampered.rest.file_sha256 = binary_hash.clone();
    fs::write(&intake_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    manifest.lineage.as_mut().unwrap().intake_sha256 =
        format!("{:x}", Sha256::digest(fs::read(&intake_path).unwrap()));
    let mut anchor = trusted_anchor(&tampered);
    anchor.rest_file_sha256 = binary_hash;
    let error = lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).unwrap_err();
    assert!(error.contains("rest record is not the exact"), "{error}");
    assert!(!error.contains("magic mismatch"), "{error}");
}

#[test]
fn rest_record_hash_must_match_its_actual_bytes() {
    let dir = root("rest-hash-mismatch");
    let (manifest, _) = staged(&dir, true);
    // Rewrite the rest file with different valid schema content so its actual
    // bytes no longer match the intake-declared file hash.
    fs::write(
        dir.join("rest"),
        rest_artifact_bytes(&format!("{};rest-drift", coarse_identity(&"a".repeat(40)))),
    )
    .unwrap();
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(
        error.contains("rest record SHA-256 does not match"),
        "{error}"
    );
}

// ---- Metadata header probes: identity bytes, clock, length, magic ----

#[test]
fn absent_states_report_lineage_absent() {
    let dir = root("absent");
    let (manifest, _) = staged(&dir, false); // rest present, no states written
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
    assert!(error.contains("48 declared N256/M512 committed states are absent"));
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let error = lineage::admit_standalone(&intake_path).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
}

#[test]
fn a_missing_rest_record_reports_lineage_absent() {
    let dir = root("rest-missing");
    let (manifest, _) = staged(&dir, true);
    fs::remove_file(dir.join("rest")).unwrap();
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
    assert!(error.contains("rest"), "{error}");
}

#[test]
fn missing_intake_file_reports_lineage_absent() {
    let dir = root("no-intake");
    let error = lineage::admit_standalone(&dir.join("lineage-intake.json")).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT));
    assert!(error.contains("no N256/M512 lineage intake exists"));
}

#[test]
fn endpoint_hash_binding_must_match_the_intake() {
    let dir = root("endpoint-binding");
    let (mut manifest, intake) = staged(&dir, true);
    let anchor = trusted_anchor(&intake);
    let admitted = lineage::admit_through_manifest_with_anchor(&manifest, Some(&anchor)).unwrap();
    assert!(lineage::bind_endpoint(&manifest, &admitted).is_ok());
    manifest.coefficient_sha256 = "7".repeat(64);
    assert!(lineage::bind_endpoint(&manifest, &admitted).is_err());
}

#[test]
fn lineage_refuses_wrong_state_clock_headers() {
    let dir = root("bad-clock");
    let (manifest, intake) = staged(&dir, true);
    let victim = &intake.states[10];
    write_state_for(victim, &manifest.identity, victim.clock - 64, 11);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("clock header mismatch"), "{error}");
}

#[test]
fn lineage_refuses_epoch_mismatch_on_the_endpoint_state() {
    let dir = root("bad-epoch");
    let (manifest, intake) = staged(&dir, true);
    write_state_for(&intake.states[47], &manifest.identity, 4096, 47);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("clock header mismatch"), "{error}");
}

#[test]
fn lineage_refuses_state_length_mismatch() {
    let dir = root("bad-length");
    let (manifest, _) = staged(&dir, true);
    let path = dir.join("step-003-clock-0192");
    let len = fs::metadata(&path).unwrap().len();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(len - 4096)
        .unwrap();
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("length mismatch"), "{error}");
}

#[test]
fn lineage_refuses_bad_magic_in_a_state_header() {
    let dir = root("bad-magic");
    let (manifest, _) = staged(&dir, true);
    let path = dir.join("step-001-clock-0064");
    let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    use std::io::{Seek, SeekFrom, Write as _};
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(b"P10AVXSNAPX\0").unwrap();
    drop(file);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("magic mismatch"), "{error}");
}

#[test]
fn lineage_refuses_one_missing_committed_state() {
    let dir = root("one-missing");
    let (manifest, _) = staged(&dir, true);
    fs::remove_file(dir.join("step-047-clock-3968")).unwrap();
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
    assert!(error.contains("1 declared"));
    assert!(error.contains("step-047-clock-3968"));
}

// ---- Identity is compared by actual bytes, not just length ----

#[test]
fn lineage_refuses_shorter_state_identity() {
    let dir = root("identity-shorter");
    let (manifest, intake) = staged(&dir, true);
    write_state_for(&intake.states[0], "shorter-identity", 64, 1);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("identity mismatch"), "{error}");
}

#[test]
fn lineage_refuses_same_length_wrong_state_identity() {
    let dir = root("identity-same-length");
    let (manifest, intake) = staged(&dir, true);
    // Same byte length as the reviewed identity, different content: a length-only
    // check (the reviewed bug) would have accepted this.
    let mut wrong = manifest.identity.clone();
    wrong.remove(0);
    wrong.push('x');
    assert_eq!(wrong.len(), manifest.identity.len());
    assert_ne!(wrong, manifest.identity);
    write_state_for(&intake.states[5], &wrong, 320, 5);
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("identity mismatch"), "{error}");
}

#[test]
fn lineage_refuses_same_length_wrong_rest_identity() {
    let dir = root("rest-same-length");
    let (mut manifest, intake) = staged(&dir, true);
    let mut wrong = coarse_identity(&"a".repeat(40));
    wrong.remove(0);
    wrong.push('x');
    assert_eq!(wrong.len(), coarse_identity(&"a".repeat(40)).len());
    let rest_hash = write_rest_artifact(&dir.join("rest"), &wrong);
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    let mut tampered = intake.clone();
    tampered.rest.file_sha256 = rest_hash;
    fs::write(&intake_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    manifest.lineage.as_mut().unwrap().intake_sha256 =
        format!("{:x}", Sha256::digest(fs::read(&intake_path).unwrap()));
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("rest record identity mismatch"), "{error}");
}

// ---- Intake structural and cross-bind refusals (unchanged semantics) ----

fn tampered_intake(dir: &Path, mutate: impl Fn(&mut LineageIntake)) -> Manifest {
    let source = "a".repeat(40);
    fs::write(
        dir.join("coarse-frozen-plan.json"),
        b"{\"synthetic\":true}\n",
    )
    .unwrap();
    let plan_sha256 = format!("{:x}", Sha256::digest(b"{\"synthetic\":true}\n"));
    let (intake_path, mut intake) = intake_fixture(dir, &source, &plan_sha256);
    mutate(&mut intake);
    fs::write(&intake_path, serde_json::to_vec_pretty(&intake).unwrap()).unwrap();
    let mut manifest = coarse_manifest(dir, &source);
    manifest.plan_sha256 = plan_sha256;
    bind_intake(&mut manifest, &intake_path);
    manifest
}

type IntakeMutation = fn(&mut LineageIntake);

#[test]
fn intake_structure_refusals() {
    let mutations: Vec<(&str, IntakeMutation, &str)> = vec![
        (
            "rest-flag",
            |value| value.from_rest = false,
            "invalid lineage intake binding",
        ),
        (
            "host",
            |value| value.host = "sulaco".into(),
            "invalid lineage intake binding",
        ),
        (
            "profile",
            |value| value.profile = "other".into(),
            "invalid lineage intake binding",
        ),
        (
            "binary-hash",
            |value| value.binary_sha256 = "z".repeat(64),
            "invalid lineage intake binding",
        ),
        (
            "schema",
            |value| value.schema = "other-schema".into(),
            "invalid lineage intake binding",
        ),
        (
            "rest-relative",
            |value| value.rest.path = std::path::PathBuf::from("relative-rest"),
            "invalid lineage intake binding",
        ),
        (
            "dropped-clock",
            |value| {
                value.states.pop();
            },
            "clocks deviate from the closed h64/h128 schedule",
        ),
        (
            "duplicated-clock",
            |value| {
                let last = value.states[47].clone();
                value.states[46] = last;
            },
            "clocks deviate from the closed h64/h128 schedule",
        ),
        (
            "off-schedule-clock",
            |value| value.states[40].clock = 2112,
            "clocks deviate from the closed h64/h128 schedule",
        ),
        (
            "state-hash",
            |value| value.states[0].coefficient_sha256 = "z".repeat(64),
            "invalid lineage state record binding",
        ),
        (
            "state-relative-path",
            |value| value.states[0].path = std::path::PathBuf::from("relative"),
            "invalid lineage state record binding",
        ),
    ];
    for (name, mutate, expected) in mutations {
        let dir = root(&format!("intake-{name}"));
        let manifest = tampered_intake(&dir, mutate);
        let error = lineage::admit_through_manifest(&manifest).unwrap_err();
        assert!(
            error.contains(expected),
            "{name}: unexpected refusal {error}"
        );
    }
}

#[test]
fn intake_cross_bind_refusals() {
    let dir = root("cross-bind");
    let manifest = tampered_intake(&dir, |value| value.source_commit = "e".repeat(40));
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("does not cross-bind"), "{error}");

    let dir = root("cross-bind-plan");
    let manifest = tampered_intake(&dir, |value| value.plan_sha256 = "9".repeat(64));
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(error.contains("does not cross-bind"), "{error}");
}

#[test]
fn tampered_intake_file_refuses_hash_binding() {
    let dir = root("tamper-hash");
    let (manifest, _) = staged(&dir, true);
    let intake_path = manifest.lineage.as_ref().unwrap().intake.clone();
    fs::write(&intake_path, b"{\"tampered\":true}\n").unwrap();
    let error = lineage::admit_through_manifest(&manifest).unwrap_err();
    assert!(
        error.contains("intake SHA-256 does not match")
            || error.contains("invalid N256/M512 lineage intake schema"),
        "{error}"
    );
}

#[test]
fn oversized_intake_refuses() {
    let dir = root("oversized-intake");
    let intake_path = dir.join("lineage-intake.json");
    fs::write(&intake_path, vec![b' '; 65 * 1024]).unwrap();
    let error = lineage::admit_standalone(&intake_path).unwrap_err();
    assert!(error.contains("exceeds 64 KiB bound"), "{error}");
}
