#![allow(clippy::too_many_lines)]
//! Coverage of the full included-decoder and model surface: the reviewed
//! adapter entry points this observer compiles but the observation flow does
//! not itself call (the M384 comparison admission, envelope and evolution
//! arms, the arithmetic-control binding, snapshot reader refusals, profile
//! bindings and compiled-inventory binding arms) are exercised here so the
//! inclusive package coverage gate covers imported code too, without weakening
//! or editing any reviewed source.
use crate::arm_tests::fixture_root;
use crate::decode;
use crate::fixtures::{self, Fixture};
use crate::model::{ComparisonKind, ProfileBinding, ProfileBindingKind};
use crate::source_bind;
use nsbu_solver::domain::Layout;
use nsbu_solver::Complex64;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn kind_fixture(name: &str, comparison_kind: &str, force: [usize; 3]) -> Fixture {
    let layout = Layout::new(N).expect("fixture layout");
    fixtures::write_fixture(
        &fixture_root(), name, &fixtures::synthetic_state(layout), comparison_kind,
        "cox-matthews", force, ELAPSED, TARGET, N, [1.0; 3], 1.0,
    )
}

fn grid_fixture(name: &str, force: [usize; 3]) -> Fixture {
    kind_fixture(name, "MATCHED_SPATIAL", force)
}

fn standard(name: &str) -> Fixture {
    grid_fixture(name, [512; 3])
}

fn manifest_variant(fixture: &Fixture, name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    let directory = fixture.manifest_path.parent().expect("directory").to_path_buf();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest")).expect("manifest");
    mutate(&mut manifest);
    let path = directory.join(format!("surface-{name}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&manifest).expect("manifest bytes")).expect("write");
    path
}

fn read_refusal(fixture: &Fixture, name: &str, fragment: &str, mutate: impl FnOnce(&mut Value)) {
    let path = manifest_variant(fixture, name, mutate);
    let error = decode::read_external_reference_manifest(&path).expect_err(name);
    assert!(error.contains(fragment), "{name}: {error}");
}

#[test]
fn time_diagnostic_profile_binds_both_reviewed_grids_and_refuses_foreign_ones() {
    for (name, force) in [("m384", [384; 3]), ("m512", [512; 3])] {
        let fixture = kind_fixture(&format!("surface-time-{name}"), "TIME_DIAGNOSTIC", force);
        decode::read_manifest(&fixture.manifest_path)
            .expect("the time-diagnostic profile admits the reviewed Cox--Matthews grids");
    }
    let foreign = kind_fixture("surface-time-m448", "TIME_DIAGNOSTIC", [448; 3]);
    let error = decode::read_manifest(&foreign.manifest_path)
        .expect_err("M448 is outside the time-diagnostic profile");
    assert!(error.contains("invalid evolution semantics"), "{error}");
}

#[test]
fn the_reviewed_m384_admission_accepts_m384_and_refuses_the_bridge_only_grids() {
    let m384 = grid_fixture("surface-m384", [384; 3]);
    decode::read_manifest(&m384.manifest_path).expect("M384 is admitted by the comparison profile");
    let m512 = standard("surface-m512");
    let error = decode::read_manifest(&m512.manifest_path).expect_err("M512 is outside the M384 profile");
    assert!(error.contains("invalid evolution semantics"), "{error}");
    let m448 = grid_fixture("surface-m448", [448; 3]);
    let error = decode::read_external_reference_manifest(&m448.manifest_path)
        .expect_err("M448 is outside the bridge profiles");
    assert!(error.contains("invalid evolution semantics"), "{error}");
}

#[test]
fn decoder_envelope_arms_refuse_malformed_bindings() {
    let fixture = standard("surface-envelope");
    read_refusal(&fixture, "foreign-schema", "invalid comparison manifest binding", |m| {
        m["schema"] = json!("p10-snapshot-comparison-input-v2");
    });
    read_refusal(&fixture, "huge-identity", "invalid comparison manifest binding", |m| {
        m["identity"] = json!(format!(
            "{};pad={}",
            m["identity"].as_str().expect("id"),
            "x".repeat((16 * 1024) + 8)
        ));
    });
    read_refusal(&fixture, "empty-backend", "invalid comparison manifest binding", |m| {
        m["backend"] = json!("");
    });
    read_refusal(&fixture, "empty-execution", "invalid comparison manifest binding", |m| {
        m["execution"] = json!("");
    });
    read_refusal(&fixture, "negative-guard", "invalid admission guard metadata", |m| {
        m["admission_guard"] = json!({"advective_limit": -1.0_f64, "maximum_attempts": 4_u64});
    });
    read_refusal(&fixture, "zero-attempts-guard", "invalid admission guard metadata", |m| {
        m["admission_guard"] = json!({"advective_limit": 1.0_f64, "maximum_attempts": 0_u64});
    });
    let guarded = manifest_variant(&fixture, "guard-ok", |m| {
        m["admission_guard"] = json!({"advective_limit": 2.5_f64, "maximum_attempts": 8_u64});
    });
    decode::read_external_reference_manifest(&guarded).expect("a sound guard binds");

    for key in ["source_commit", "plan_sha256", "coefficient_sha256", "file_sha256"] {
        read_refusal(&fixture, &format!("unhex-{key}"), "invalid comparison manifest binding", move |m| {
            m[key] = json!("z".repeat(if key == "source_commit" { 40 } else { 64 }));
        });
    }
    read_refusal(&fixture, "unhex-case", "invalid comparison manifest binding", |m| {
        m["evolution"]["case_sha256"] = json!("z".repeat(64));
    });
    let oversized = manifest_variant(&fixture, "oversized", |m| {
        m["plan"] = json!(format!("{}.json", "p".repeat(70 * 1024)));
    });
    let error = decode::read_external_reference_manifest(&oversized).expect_err("oversized manifest");
    assert!(error.contains("manifest exceeds 64 KiB bound"), "{error}");
}

#[test]
fn decoder_evolution_and_schedule_arms_refuse_every_inconsistency() {
    let fixture = standard("surface-evolution");
    read_refusal(&fixture, "target-drift", "invalid evolution semantics", |m| {
        m["target"] = json!(9000_u64);
    });
    read_refusal(&fixture, "endpoint-drift", "invalid evolution semantics", |m| {
        m["evolution"]["comparison_endpoint"] = json!(2048_u128);
    });
    read_refusal(&fixture, "empty-schedule", "invalid evolution semantics", |m| {
        m["evolution"]["schedule"] = json!([]);
    });
    read_refusal(&fixture, "foreign-method", "invalid evolution semantics", |m| {
        m["evolution"]["method"] = json!("hochbruck-ostermann");
    });
    read_refusal(&fixture, "tolerance-zero", "invalid evolution tolerances", |m| {
        m["evolution"]["absolute_tolerances"] = json!([0.0_f64, 1e-4]);
    });
    read_refusal(&fixture, "relative-tolerance-negative", "invalid evolution tolerances", |m| {
        m["evolution"]["relative_tolerances"] = json!([-1.0_f64, 1e-4]);
    });
    for (name, segment, fragment) in [
        (
            "gap",
            json!([
                {"from_inclusive": 0_u128, "until_exclusive": 512_u128, "step_ticks": 512_u128},
                {"from_inclusive": 768_u128, "until_exclusive": 1024_u128, "step_ticks": 256_u128},
            ]),
            "invalid piecewise schedule",
        ),
        (
            "reversed",
            json!([{"from_inclusive": 0_u128, "until_exclusive": 0_u128, "step_ticks": 512_u128}]),
            "invalid piecewise schedule",
        ),
        (
            "zero-step",
            json!([{"from_inclusive": 0_u128, "until_exclusive": 1024_u128, "step_ticks": 0_u128}]),
            "invalid piecewise schedule",
        ),
        (
            "ragged",
            json!([{"from_inclusive": 0_u128, "until_exclusive": 1024_u128, "step_ticks": 3_u128}]),
            "invalid piecewise schedule",
        ),
        (
            "short",
            json!([{"from_inclusive": 0_u128, "until_exclusive": 768_u128, "step_ticks": 256_u128}]),
            "piecewise schedule does not reach target",
        ),
    ] {
        let segment = segment.clone();
        read_refusal(&fixture, name, fragment, move |m| {
            m["evolution"]["schedule"] = segment.clone();
        });
    }
}

fn arithmetic_review(conclusion: &str, serial_configuration: &str, force_dims: [u64; 3]) -> Value {
    let lineage_side = json!({
        "source_commit": fixtures::fixture_source_commit(),
        "backend": "rustfft-6.4.1-avx-avx2-fma",
        "execution": "serial",
        "profile": {"kind": "identity-profile-field", "value": "surface-arithmetic"},
    });
    json!({
        "schema": "p10-time-arithmetic-review-v1",
        "conclusion": conclusion,
        "case_sha256": nsbu_benchmarks::CASE_SHA256,
        "method": "cox-matthews",
        "integration_force_dimensions": force_dims,
        "measured_control": {
            "outcome": "successful-exact-bit",
            "serial": {
                "source_commit": fixtures::fixture_source_commit(),
                "backend": "rustfft-6.4.1-avx-avx2-fma",
                "execution": "serial",
                "configuration": serial_configuration,
            },
            "w3": {
                "source_commit": fixtures::fixture_source_commit(),
                "backend": "rustfft-6.4.1-avx-avx2-fma",
                "execution": "w3",
                "configuration": "w3",
            },
        },
        "reviewed_lineage": {
            "status": "reviewed-unchanged-kernel-lineage",
            "left_control_role": "serial",
            "right_control_role": "w3",
            "left": lineage_side,
            "right": lineage_side,
        },
    })
}

fn attach_arithmetic_control(
    fixture: &Fixture,
    name: &str,
    manifest_review: &Value,
    evidence_review: &Value,
    evidence_sha256: Option<&str>,
) -> PathBuf {
    let directory = fixture.manifest_path.parent().expect("directory");
    let evidence_path = directory.join(format!("surface-{name}-evidence.json"));
    fs::write(&evidence_path, serde_json::to_vec_pretty(evidence_review).expect("evidence")).expect("write");
    let digest = format!("{:x}", Sha256::digest(fs::read(&evidence_path).expect("evidence bytes")));
    let evidence_name = evidence_path.file_name().expect("name").to_string_lossy().into_owned();
    let sha = evidence_sha256.map(str::to_owned).unwrap_or(digest);
    manifest_variant(fixture, name, move |m| {
        m["arithmetic_control"] = json!({
            "evidence": evidence_name,
            "evidence_sha256": sha,
            "review": manifest_review,
        });
    })
}

#[test]
fn arithmetic_control_binds_and_refuses_every_documented_divergence() {
    let fixture = standard("surface-arithmetic");
    let sound = arithmetic_review("reviewed-equivalence-supported-by-controls", "serial", [512, 512, 512]);

    let bound = attach_arithmetic_control(&fixture, "bound", &sound, &sound, None);
    decode::read_external_reference_manifest(&bound).expect("the full arithmetic control binds");

    // The review must sit on the manifest's own evolution force grid: an
    // M384 review on the M512 fixture is refused, and an M384 review binds
    // on the M384 fixture.
    let offgrid = arithmetic_review(
        "reviewed-equivalence-supported-by-controls",
        "serial",
        [384, 384, 384],
    );
    let mismatched_grid = attach_arithmetic_control(&fixture, "grid-mismatch", &offgrid, &offgrid, None);
    let error = decode::read_external_reference_manifest(&mismatched_grid)
        .expect_err("a 384 review cannot vouch for a 512 evolution");
    assert!(error.contains("invalid arithmetic-control binding"), "{error}");
    let m384 = grid_fixture("surface-arithmetic-m384", [384; 3]);
    let m384_sound = arithmetic_review(
        "reviewed-equivalence-supported-by-controls",
        "serial",
        [384, 384, 384],
    );
    let m384_bound = attach_arithmetic_control(&m384, "bound", &m384_sound, &m384_sound, None);
    decode::read_external_reference_manifest(&m384_bound).expect("the M384 arithmetic control binds");

    let stale = attach_arithmetic_control(&fixture, "stale-digest", &sound, &sound, Some(&"0".repeat(64)));
    let error = decode::read_external_reference_manifest(&stale).expect_err("stale evidence digest");
    assert!(error.contains("arithmetic-control SHA-256 mismatch"), "{error}");

    let unhex = attach_arithmetic_control(&fixture, "unhex-digest", &sound, &sound, Some(&"z".repeat(64)));
    let error = decode::read_external_reference_manifest(&unhex).expect_err("unhex evidence digest");
    assert!(error.contains("invalid arithmetic-control binding"), "{error}");

    let foreign = attach_arithmetic_control(
        &fixture,
        "foreign-schema",
        &arithmetic_review("unsupported-conclusion", "serial", [512, 512, 512]),
        &arithmetic_review("unsupported-conclusion", "serial", [512, 512, 512]),
        None,
    );
    let error = decode::read_external_reference_manifest(&foreign).expect_err("foreign review");
    assert!(error.contains("invalid arithmetic-control binding"), "{error}");

    // A valid-but-different evidence document, correctly digested, is a
    // structural content mismatch against the manifest's review object.
    let divergent =
        arithmetic_review("reviewed-equivalence-supported-by-controls", "serial-evidence-side", [512, 512, 512]);
    let mismatched = attach_arithmetic_control(&fixture, "content", &sound, &divergent, None);
    let error = decode::read_external_reference_manifest(&mismatched).expect_err("content mismatch");
    assert!(error.contains("arithmetic-control evidence content mismatch"), "{error}");

    let malformed = manifest_variant(&fixture, "malformed-evidence", |m| {
        m["arithmetic_control"] = json!({
            "evidence": "surface-missing-evidence.json",
            "evidence_sha256": "0".repeat(64),
            "review": sound,
        });
    });
    let error = decode::read_external_reference_manifest(&malformed).expect_err("missing evidence");
    assert!(!error.is_empty(), "{error}");
}

/// Rewrites the snapshot file (and optionally the coefficient digest the
/// manifest claims for it) so each reader arm fires in isolation.
fn snapshot_variant(
    fixture: &Fixture,
    name: &str,
    rebinding_digests: bool,
    produce: impl FnOnce(&str) -> (Vec<u8>, Option<String>),
) -> PathBuf {
    let (bytes, coefficient_sha256) = produce(&fixture.identity);
    let directory = fixture.manifest_path.parent().expect("directory");
    let snapshot_path = directory.join(format!("surface-{name}.bin"));
    fs::write(&snapshot_path, &bytes).expect("snapshot write");
    manifest_variant(fixture, name, move |m| {
        m["snapshot"] = json!(snapshot_path.file_name().expect("name").to_string_lossy());
        m["file_sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
        if rebinding_digests {
            m["coefficient_sha256"] = json!(coefficient_sha256.expect("coefficient digest"));
        }
    })
}

fn unbound_snapshot(bytes: Vec<u8>) -> (Vec<u8>, Option<String>) {
    (bytes, None)
}

fn base_snapshot(identity: &str) -> (Vec<u8>, String) {
    let layout = Layout::new(N).expect("layout");
    fixtures::snapshot_bytes(identity, ELAPSED, TARGET, &fixtures::synthetic_state(layout))
}

#[test]
fn snapshot_reader_arms_refuse_every_malformed_field() {
    fn load_refuse(
        fixture: &Fixture,
        name: &str,
        fragment: &str,
        rebinding_digests: bool,
        produce: impl FnOnce(&str) -> (Vec<u8>, Option<String>),
    ) {
        let path = snapshot_variant(fixture, name, rebinding_digests, produce);
        let manifest = decode::read_external_reference_manifest(&path).expect("manifest binds");
        let error = decode::load(&manifest).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }
    let fixture = standard("surface-snapshot");
    load_refuse(&fixture, "bad-magic", "snapshot magic mismatch", false, |identity| {
        let (mut bytes, _) = base_snapshot(identity);
        bytes[3] = b'X';
        unbound_snapshot(bytes)
    });
    load_refuse(&fixture, "identity-length", "snapshot identity length mismatch", false, |identity| {
        let (mut bytes, _) = base_snapshot(identity);
        bytes[12] ^= 1;
        unbound_snapshot(bytes)
    });
    load_refuse(&fixture, "identity-bytes", "snapshot identity mismatch", false, |identity| {
        let (mut bytes, _) = base_snapshot(identity);
        bytes[20] ^= 0x20;
        unbound_snapshot(bytes)
    });
    load_refuse(&fixture, "clock-mismatch", "snapshot clock mismatch", false, |identity| {
        let layout = Layout::new(N).expect("layout");
        unbound_snapshot(
            fixtures::snapshot_bytes(identity, ELAPSED + 1, TARGET, &fixtures::synthetic_state(layout)).0,
        )
    });
    load_refuse(&fixture, "trailer", "coefficient SHA-256 trailer mismatch", false, |identity| {
        let (mut bytes, _) = base_snapshot(identity);
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        unbound_snapshot(bytes)
    });
    // A valid snapshot of different coefficients keeps file length and trailer
    // self-consistent; the manifest's stale digests trigger the hash refusal.
    load_refuse(&fixture, "hash-mismatch", "snapshot hash does not match reviewed manifest", false, |identity| {
        let layout = Layout::new(N).expect("layout");
        let mut coefficients = fixtures::synthetic_state(layout);
        coefficients[1][1] = Complex64::new(0.5, -0.25);
        unbound_snapshot(fixtures::snapshot_bytes(identity, ELAPSED, TARGET, &coefficients).0)
    });
    // A non-Hermitian k=0 coefficient, fully self-consistent, is refused only
    // by the spectrum validation inside load.
    load_refuse(&fixture, "spectrum", "InvalidSpectrum", true, |identity| {
        let layout = Layout::new(N).expect("layout");
        let mut coefficients = fixtures::synthetic_state(layout);
        coefficients[0][0] = Complex64::new(0.0, 0.5);
        let (bytes, coefficient_sha256) =
            fixtures::snapshot_bytes(identity, ELAPSED, TARGET, &coefficients);
        (bytes, Some(coefficient_sha256))
    });
    let padded_identity = manifest_variant(&fixture, "length-mismatch", |m| {
        m["identity"] = json!(format!("{};pad={}", m["identity"].as_str().expect("id"), "y".repeat(64)));
    });
    let manifest = decode::read_external_reference_manifest(&padded_identity).expect("manifest binds");
    let error = decode::load(&manifest).expect_err("length mismatch");
    assert!(error.contains("snapshot length mismatch"), "{error}");
}

#[test]
fn reader_bounds_and_admission_helpers_are_exercised() {
    let fixture = standard("surface-reader-bounds");
    let manifest = decode::read_external_reference_manifest(&fixture.manifest_path).expect("binds");
    let bytes = decode::state_bytes(&manifest).expect("state bytes");
    assert!(bytes > 0);
    let admitted = decode::admitted_bytes(&manifest, &manifest).expect("admission bound");
    assert!(admitted > bytes);
    let three = decode::admitted_bytes_three(&manifest, &manifest, &manifest).expect("three-way bound");
    assert!(three > admitted);
    decode::preflight(&manifest, &manifest).expect("lengths match");
    decode::preflight_three(&manifest, &manifest, &manifest).expect("lengths match");

    let oversized_plan = manifest_variant(&fixture, "plan-oversize", |m| {
        m["plan"] = json!("surface-oversize-plan.json");
    });
    let plan_path = oversized_plan.parent().expect("parent").join("surface-oversize-plan.json");
    fs::write(&plan_path, vec![b' '; (1024 * 1024) + 8]).expect("oversized plan");
    let mut rebind: Value =
        serde_json::from_slice(&fs::read(&oversized_plan).expect("manifest")).expect("manifest");
    let digest = format!("{:x}", Sha256::digest(fs::read(&plan_path).expect("plan")));
    rebind["plan_sha256"] = json!(digest);
    let path = oversized_plan.parent().expect("parent").join("surface-plan-oversize2.json");
    fs::write(&path, serde_json::to_vec_pretty(&rebind).expect("bytes")).expect("write");
    let error = decode::read_external_reference_manifest(&path).expect_err("oversized plan");
    assert!(error.contains("plan exceeds 1 MiB bound"), "{error}");
}

#[test]
fn profile_bindings_compare_identity_and_profile_field_forms() {
    let legacy = ProfileBinding {
        kind: ProfileBindingKind::LegacyFullIdentity,
        value: "full-identity".to_owned(),
    };
    assert!(legacy.matches_identity("full-identity"));
    assert!(!legacy.matches_identity("other"));
    let field = ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: "profile-value".to_owned(),
    };
    assert!(field.matches_identity("descriptor;profile=profile-value;case=x"));
    assert!(!field.matches_identity("descriptor;profile=other"));
    assert!(!ProfileBinding {
        kind: ProfileBindingKind::IdentityProfileField,
        value: String::new(),
    }
    .matches_identity("profile="));
    assert_eq!(ComparisonKind::default(), ComparisonKind::MatchedSpatial);
}

#[test]
fn compiled_bindings_refuse_foreign_inventories() {
    let sound: Value = serde_json::from_slice(source_bind::SEALED_INVENTORY).expect("compiled inventory");
    source_bind::dependency_aggregate(&sound, "nsbu-solver").expect("sound aggregate");
    for foreign in [
        json!({"path_dependencies": []}),
        json!({"path_dependencies": [{"crate": "nsbu-solver", "files": "not-an-array"}]}),
        json!({"path_dependencies": [{"crate": "nsbu-solver", "files": [{"repository_path": "x"}]}]}),
        json!({"path_dependencies": [{"crate": "nsbu-solver", "files": [{"repository_path": "x", "sha256": "y"}], "aggregate_sha256": "wrong"}]}),
    ] {
        assert!(source_bind::dependency_aggregate(&foreign, "nsbu-solver").is_err());
    }
    let error = source_bind::dependency_aggregate(&sound, "nonexistent-crate")
        .expect_err("unknown dependency");
    assert!(error.contains("carries no nonexistent-crate dependency"), "{error}");

    let mut broken_digest = sound.clone();
    broken_digest["included_sources"][0]["sha256"] = json!("f".repeat(64));
    let error = source_bind::bindings_for(&broken_digest)
        .expect_err("compiled bytes must match the sealed inventory digest");
    assert!(error.contains("differ from the sealed inventory digest"), "{error}");

    let mut no_role = sound.clone();
    no_role["included_sources"] = json!([]);
    let error = source_bind::bindings_for(&no_role).expect_err("roles must be sealed");
    assert!(error.contains("carries no sealed digest"), "{error}");

    let mut broken_aggregate = sound.clone();
    broken_aggregate["path_dependencies"][0]["aggregate_sha256"] = json!("e".repeat(64));
    let error = source_bind::bindings_for(&broken_aggregate).expect_err("aggregates must cross-check");
    assert!(error.contains("does not match its sealed value"), "{error}");

    source_bind::bindings_for(&sound).expect("the sealed inventory binds");
}
