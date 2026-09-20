#![allow(clippy::too_many_lines)]
//! Refusal-arm coverage: bounded reads and strict parsers, manifest path
//! confinement and the plan semantic variant arms.
use crate::fixtures::{self, fixture_source_commit, Fixture};
use crate::{decode, execute, provenance, Mode, Request};
use nsbu_solver::domain::Layout;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::{Path, PathBuf}};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-coverage")
}

fn request() -> Request {
    Request {
        velocity_samples: 16,
        pressure_samples: 16,
        force_samples: 16,
        workers: 1,
        root_budget: 32,
        max_reference_evaluations: 1 << 24,
        cap: 1 << 30,
        velocity_floor: 1e-12,
        pressure_floor: 1e-12,
        backend: "owned-radix".to_owned(),
    }
}

fn fixture(name: &str) -> Fixture {
    let layout = Layout::new(N).expect("fixture layout");
    let coefficients = fixtures::synthetic_state(layout);
    fixtures::write_fixture(
        &fixture_root(),
        name,
        &coefficients,
        "MATCHED_SPATIAL",
        "cox-matthews",
        [512; 3],
        ELAPSED,
        TARGET,
        N,
        [1.0; 3],
        1.0,
    )
}

fn plan_variant(fixture: &Fixture, name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    let directory = fixture
        .manifest_path
        .parent()
        .expect("fixture directory")
        .to_path_buf();
    let mut plan: Value =
        serde_json::from_slice(&fs::read(directory.join("plan.json")).expect("plan")).expect("plan");
    mutate(&mut plan);
    let plan_bytes = serde_json::to_vec_pretty(&plan).expect("plan bytes");
    let plan_name = format!("{name}.plan.json");
    fs::write(directory.join(&plan_name), &plan_bytes).expect("plan write");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest")).expect("manifest");
    manifest["plan"] = json!(plan_name);
    manifest["plan_sha256"] = json!(format!("{:x}", Sha256::digest(&plan_bytes)));
    let manifest_path = directory.join(format!("{name}.manifest.json"));
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).expect("manifest bytes"))
        .expect("manifest write");
    manifest_path
}

fn run_refusal(fixture: &Fixture, name: &str, fragment: &str, mutate: impl FnOnce(&mut Value)) {
    let path = plan_variant(fixture, name, mutate);
    let error = execute(Mode::Run, &path, request(), None).expect_err(name);
    assert!(error.contains(fragment), "{name}: {error}");
}


#[test]
fn bounded_reads_and_strict_parsers_refuse_adversarial_input() {
    let directory = fixture_root().join("reads");
    fs::create_dir_all(&directory).expect("directory");

    let error = provenance::read_text(&directory.join("absent.json"), 64)
        .expect_err("missing manifest must refuse");
    assert!(error.contains("read failure"), "{error}");

    let oversized = directory.join("oversized.json");
    fs::write(&oversized, vec![b' '; 64]).expect("write");
    let error = provenance::read_text(&oversized, 32).expect_err("oversized file must refuse");
    assert!(error.contains("exceeds 32 bytes"), "{error}");

    let binary = directory.join("binary.json");
    fs::write(&binary, [0xff_u8, 0xfe, 0xfd]).expect("write");
    let error = provenance::read_text(&binary, 64).expect_err("non-UTF-8 must refuse");
    assert!(error.contains("not valid UTF-8"), "{error}");

    for (name, text, fragment) in [
        ("empty", "", "empty JSON input"),
        ("unterminated", "{\"a\":", "unterminated JSON structure"),
        ("trailing", "{} trailing", "trailing JSON content"),
        ("unterminated-string", "{\"a", "unterminated string"),
        ("unbalanced", "}", "unbalanced JSON"),
        ("array", "[1,2]", "JSON object"),
    ] {
        let error = provenance::parse_strict(text).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }

    for (name, identity, fragment) in [
        ("empty-field", "desc;;a=1", "empty field"),
        ("no-delimiter", "desc;a", "carries no '='"),
        ("empty-key", "desc;=1", "empty key"),
        ("duplicate", "desc;a=1;a=2", "duplicate field"),
    ] {
        let error = provenance::identity_fields_strict(identity).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }
    let fields = provenance::identity_fields_strict("descriptor-only").expect("descriptor");
    assert_eq!(fields[0].0, "@descriptor");
}

#[test]
fn manifest_paths_must_stay_inside_the_manifest_directory() {
    let fixture = fixture("paths");
    let directory = fixture.manifest_path.parent().expect("directory");
    fs::create_dir_all(directory.join("sub")).expect("subdirectory");
    for (name, plan_value) in [
        (
            "absolute",
            directory.join("plan.json").to_string_lossy().into_owned(),
        ),
        ("parent", "sub/../plan.json".to_owned()),
    ] {
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest")).expect("manifest");
        manifest["plan"] = json!(plan_value);
        let path = directory.join(format!("path-{name}.manifest.json"));
        fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).expect("manifest");
        let error = execute(Mode::Run, &path, request(), None).expect_err(name);
        assert!(error.contains("manifest directory"), "{name}: {error}");
    }
}

#[test]
fn plan_source_and_endpoint_variant_arms_refuse() {
    let fixture = fixture("plan-arms");
    run_refusal(&fixture, "harness-source-conflict", "harness run source", |plan| {
        plan["harness_commit_and_run_source"] = json!("a".repeat(40));
    });
    run_refusal(&fixture, "plan-production-missing-field", "production_source", |plan| {
        plan["production_source_commit"] = json!("c".repeat(40));
    });
    run_refusal(&fixture, "plan-test-missing-field", "test_source", |plan| {
        plan["numerical_test_source_commit"] = json!("d".repeat(40));
    });
    run_refusal(&fixture, "plan-endpoint-differs", "plan endpoint differs", |plan| {
        plan["endpoint_ticks"] = json!(TARGET + 1);
    });
}

#[test]
fn plan_schedule_variant_arms_refuse() {
    let fixture = fixture("plan-arms-schedule");
    run_refusal(&fixture, "plan-empty-schedule", "schedule is empty", |plan| {
        plan["schedule"] = json!([]);
    });
    for (name, segment) in [
        ("zero-step", json!({"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 0_u64})),
        ("past-endpoint", json!({"from_inclusive": 0_u64, "until_exclusive": 9000_u64, "step_ticks": 2_u64})),
        ("missing-step", json!({"from_inclusive": 0_u64, "until_exclusive": 1024_u64})),
    ] {
        run_refusal(&fixture, name, "plan schedule", |plan| {
            plan["schedule"] = json!([segment.clone()]);
        });
    }
    run_refusal(&fixture, "unordered-schedule", "plan schedule", |plan| {
        plan["schedule"] = json!([
            {"from_inclusive": 2048_u64, "until_exclusive": 3072_u64, "step_ticks": 2_u64},
            {"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 2_u64},
        ]);
    });
}

#[test]
fn plan_source_harness_key_binds_and_missing_endpoint_refuses() {
    let fixture = fixture("plan-arms-sources");
    // Accepting arm: a harness-key-only source still declares the commit.
    let harness_only = plan_variant(&fixture, "harness-source-ok", |plan| {
        let object = plan.as_object_mut().expect("plan object");
        object.remove("source_commit");
        object.insert(
            "harness_commit_and_run_source".to_owned(),
            json!(fixture_source_commit()),
        );
    });
    execute(Mode::Run, &harness_only, request(), None).expect("harness-key source binds");
    // The endpoint tick is a required field; omitting it is refused even when
    // the identity alone would carry the same endpoint.
    let missing_endpoint = plan_variant(&fixture, "no-endpoint-refused", |plan| {
        let object = plan.as_object_mut().expect("plan object");
        object.remove("endpoint_ticks");
    });
    let error = execute(Mode::Run, &missing_endpoint, request(), None)
        .expect_err("a plan without endpoint_ticks must be refused");
    assert!(error.contains("endpoint_ticks"), "{error}");
    assert!(error.contains("missing"), "{error}");
}

/// Rebuild the snapshot+manifest with the identity extended by extra fields so
/// plan/identity production-source agreement arms become reachable.
fn identity_extension(fixture: &Fixture, name: &str, extra: &str) -> PathBuf {
    let directory = fixture.manifest_path.parent().expect("directory");
    let snapshot = fs::read(directory.join("snapshot.bin")).expect("snapshot");
    let body = &snapshot[12 + 8 + fixture.identity.len()..];
    let identity = format!("{};{extra}", fixture.identity);
    let mut rebuilt = Vec::new();
    rebuilt.extend(&snapshot[..12]);
    rebuilt.extend((identity.len() as u64).to_le_bytes());
    rebuilt.extend(identity.as_bytes());
    rebuilt.extend(body);
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest")).expect("manifest");
    manifest["identity"] = json!(identity);
    manifest["file_sha256"] = json!(format!("{:x}", Sha256::digest(&rebuilt)));
    let manifest_name = format!("{name}.manifest.json");
    fs::write(directory.join(&manifest_name), serde_json::to_vec_pretty(&manifest).unwrap())
        .expect("manifest");
    // Re-point the plan onto the same digest-bound body it already carries.
    let plan_bytes = fs::read(directory.join("plan.json")).expect("plan");
    let mut manifest: Value = serde_json::from_slice(
        &fs::read(directory.join(&manifest_name)).expect("manifest"),
    )
    .expect("manifest");
    manifest["plan_sha256"] = json!(format!("{:x}", Sha256::digest(&plan_bytes)));
    fs::write(directory.join(&manifest_name), serde_json::to_vec_pretty(&manifest).unwrap())
        .expect("manifest");
    fs::write(directory.join(format!("{name}.snapshot.bin")), &rebuilt).expect("snapshot");
    manifest["snapshot"] = json!(format!("{name}.snapshot.bin"));
    fs::write(directory.join(&manifest_name), serde_json::to_vec_pretty(&manifest).unwrap())
        .expect("manifest");
    directory.join(manifest_name)
}

#[test]
fn identity_extension_binds_matching_plan_sources_and_refuses_foreign_ones() {
    let fixture = fixture("extension");
    let production = "e".repeat(40);
    let base = identity_extension(
        &fixture,
        "extended",
        &format!("production_source={production};test_source={}", "d".repeat(40)),
    );
    // matching production/test sources bind
    let matching = plan_variant_from(&base, "extended-match", |plan| {
        plan["production_source_commit"] = json!(&production);
        plan["numerical_test_source_commit"] = json!("d".repeat(40));
    });
    execute(Mode::Preflight, &matching, request(), None).expect("matching extension binds");
    // a foreign plan source against a present identity field is refused
    let foreign = plan_variant_from(&base, "extended-foreign", |plan| {
        plan["production_source_commit"] = json!("f".repeat(40));
    });
    let error = execute(Mode::Run, &foreign, request(), None).expect_err("foreign production");
    assert!(error.contains("production_source"), "{error}");
}

fn plan_variant_from(manifest_path: &Path, name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    let directory = manifest_path.parent().expect("directory").to_path_buf();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(manifest_path).expect("manifest")).expect("manifest");
    let plan_name = manifest["plan"].as_str().expect("plan name").to_owned();
    let mut plan: Value =
        serde_json::from_slice(&fs::read(directory.join(&plan_name)).expect("plan")).expect("plan");
    mutate(&mut plan);
    let plan_bytes = serde_json::to_vec_pretty(&plan).expect("plan bytes");
    let new_plan = format!("{name}.plan.json");
    fs::write(directory.join(&new_plan), &plan_bytes).expect("plan write");
    manifest["plan"] = json!(new_plan);
    manifest["plan_sha256"] = json!(format!("{:x}", Sha256::digest(&plan_bytes)));
    let path = directory.join(format!("{name}.variant.json"));
    fs::write(&path, serde_json::to_vec_pretty(&manifest).unwrap()).expect("manifest");
    path
}

#[test]
fn every_admitted_comparison_kind_binds_through_the_m384_reader() {
    let layout = Layout::new(N).expect("fixture layout");
    let m512_diagnostic = fixtures::write_fixture(
        &fixture_root(),
        "surface-m512-diagnostic",
        &fixtures::synthetic_state(layout),
        "MATCHED_M512_SPATIAL_DIAGNOSTIC",
        "cox-matthews",
        [512; 3],
        ELAPSED,
        TARGET,
        N,
        [1.0; 3],
        1.0,
    );
    decode::read_manifest(&m512_diagnostic.manifest_path).expect("M512 spatial diagnostic binds");
    let force_resolution = fixtures::write_fixture(
        &fixture_root(),
        "surface-force-resolution",
        &fixtures::synthetic_state(layout),
        "FORCE_RESOLUTION_DIAGNOSTIC",
        "cox-matthews",
        [384; 3],
        ELAPSED,
        TARGET,
        N,
        [1.0; 3],
        1.0,
    );
    decode::read_manifest(&force_resolution.manifest_path).expect("force-resolution profile binds");
    let method_diagnostic = fixtures::write_fixture(
        &fixture_root(),
        "surface-method-diagnostic",
        &fixtures::synthetic_state(layout),
        "METHOD_DIAGNOSTIC",
        "hochbruck-ostermann",
        [384; 3],
        ELAPSED,
        TARGET,
        N,
        [1.0; 3],
        1.0,
    );
    decode::read_manifest(&method_diagnostic.manifest_path).expect("HO method profile binds");
    fn local_manifest_variant(
        fixture: &crate::fixtures::Fixture,
        name: &str,
        mutate: impl FnOnce(&mut Value),
    ) -> std::path::PathBuf {
        let directory = fixture.manifest_path.parent().expect("directory").to_path_buf();
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&fixture.manifest_path).expect("manifest"))
            .expect("manifest");
        mutate(&mut manifest);
        let path = directory.join(format!("coverage-{name}.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&manifest).expect("bytes")).expect("write");
        path
    }
    let mismatched = local_manifest_variant(&method_diagnostic, "method-mismatch", |m| {
        m["evolution"]["integration_force_dimensions"] = json!([512, 512, 512]);
    });
    let error = decode::read_manifest(&mismatched).expect_err("HO requires the M384 grid");
    assert!(error.contains("invalid evolution semantics"), "{error}");
}
