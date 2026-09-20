#![allow(clippy::too_many_lines)]
//! Real CLI mutation tests: every mutation is applied to the plan or clock
//! record beside a valid fixture, the manifest binding is re-digested (an
//! attacker who re-hashes consistently must still be refused), and the refusal
//! is produced through the actual command-line entry point with the exact
//! expected message fragment asserted.
use crate::fixtures::{self, clock_record_json, fixture_source_commit, Fixture};
use crate::plan::APPROVED_PLAN_SCHEMA;
use crate::{command, publication, publication::PublicationFault};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-mutation")
}

fn fixture(name: &str) -> Fixture {
    let layout = nsbu_solver::domain::Layout::new(N).expect("fixture layout");
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

/// Tamper the plan, re-point and re-digest the manifest at it, then run the
/// real CLI. The re-hashed binding must not save the semantically wrong plan.
fn cli_plan_refusal(fixture: &Fixture, name: &str, fragment: &str, mutate: impl FnOnce(&mut Value)) {
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
    let error = cli_preflight(&manifest_path).expect_err(name);
    assert!(error.contains(fragment), "{name}: {error}");
}

fn cli_preflight(manifest_path: &std::path::Path) -> Result<String, String> {
    command(vec![
        "preflight".into(),
        manifest_path.to_string_lossy().into_owned(),
        "16".into(),
        "16".into(),
        "16".into(),
        "1".into(),
        "32".into(),
        "16777216".into(),
        "1073741824".into(),
        "1e-12".into(),
        "1e-12".into(),
        "owned-radix".into(),
    ])
}

fn cli_ledger(record_path: &std::path::Path) -> Result<String, String> {
    command(vec![
        "n512-ledger".into(),
        record_path.to_string_lossy().into_owned(),
        "512".into(),
        "1024".into(),
        "1024".into(),
        "32".into(),
        "1099511627776".into(),
        "281474976710656".into(),
    ])
}

#[test]
fn cli_refuses_wrong_typed_and_rehashed_plan_schemas() {
    let fixture = fixture("schema-mutations");
    cli_plan_refusal(&fixture, "schema-wrong-string", "not the approved", |plan| {
        plan["schema"] = json!("p10-n512-m512-endpoint-capture-plan-v1");
    });
    cli_plan_refusal(&fixture, "schema-forged-revision", "not the approved", |plan| {
        plan["schema"] = json!("{APPROVED_PLAN_SCHEMA}-unapproved-revision");
    });
    cli_plan_refusal(&fixture, "schema-numeric", "\"schema\" is present but is not a string", |plan| {
        plan["schema"] = json!(1);
    });
    cli_plan_refusal(&fixture, "schema-missing", "\"schema\" is missing", |plan| {
        plan.as_object_mut().expect("object").remove("schema");
    });
    cli_plan_refusal(&fixture, "source-numeric", "\"source_commit\" is present but is not a string", |plan| {
        plan["source_commit"] = json!(1_234_567_890_u64);
    });
    cli_plan_refusal(&fixture, "profile-wrong-string", "plan profile differs", |plan| {
        plan["profile"] = json!("rehashed-by-attacker");
    });
    cli_plan_refusal(&fixture, "qualification-numeric", "qualification must be false", |plan| {
        plan["qualification"] = json!(0);
    });
    cli_plan_refusal(&fixture, "artifact-hash-numeric", "\"binary_sha256\" is present but is not a string", |plan| {
        plan["binary_sha256"] = json!(7);
    });
    cli_plan_refusal(&fixture, "artifact-hash-short-string", "binary_sha256 is not 64 lowercase", |plan| {
        plan["binary_sha256"] = json!("abc");
    });
}

#[test]
fn cli_refuses_wrong_typed_layouts_endpoints_nodes_and_schedules() {
    let fixture = fixture("field-mutations");
    for (name, fragment, mutate) in [
        (
            "retained-string",
            "\"retained_layout\" is present but is not an integer",
            Box::new(|plan: &mut Value| plan["retained_layout"] = json!("512"))
                as Box<dyn Fn(&mut Value)>,
        ),
        (
            "retained-foreign",
            "retained layout differs",
            Box::new(|plan: &mut Value| plan["retained_layout"] = json!(512)),
        ),
        (
            "force-numeric-hash",
            "\"integration_force_layout\" is present but is not an integer",
            Box::new(|plan: &mut Value| plan["integration_force_layout"] = json!(384.5)),
        ),
        (
            "endpoint-string",
            "\"endpoint_ticks\" is present but is not an integer",
            Box::new(|plan: &mut Value| plan["endpoint_ticks"] = json!("8192")),
        ),
        (
            "endpoint-rehashed",
            "plan endpoint differs from the identity endpoint field",
            Box::new(|plan: &mut Value| plan["endpoint_ticks"] = json!(TARGET + 1)),
        ),
        (
            "nodes-missing",
            "carries no observer_nodes array",
            Box::new(|plan: &mut Value| {
                plan.as_object_mut().expect("object").remove("observer_nodes");
            }),
        ),
        (
            "nodes-fractional",
            "observer node is present but is not an integer",
            Box::new(|plan: &mut Value| plan["observer_nodes"] = json!([0, 1024.5])),
        ),
        (
            "nodes-descending",
            "strictly ascending",
            Box::new(|plan: &mut Value| plan["observer_nodes"] = json!([1024, 0])),
        ),
        (
            "schedule-missing",
            "carries no schedule array",
            Box::new(|plan: &mut Value| {
                plan.as_object_mut().expect("object").remove("schedule");
            }),
        ),
        (
            "segment-step-string",
            "\"step_ticks\" is present but is not an integer",
            Box::new(|plan: &mut Value| {
                plan["schedule"] = json!([{"from_inclusive": 0, "until_exclusive": TARGET, "step_ticks": "512"}]);
            }),
        ),
        (
            "segment-from-float",
            "\"from_inclusive\" is present but is not an integer",
            Box::new(|plan: &mut Value| {
                plan["schedule"] = json!([{"from_inclusive": 0.5, "until_exclusive": TARGET, "step_ticks": 512}]);
            }),
        ),
    ] {
        cli_plan_refusal(&fixture, name, fragment, mutate);
    }
}

#[test]
fn cli_ledger_refuses_wrong_typed_and_missing_canonical_source_commit() {
    fs::create_dir_all(fixture_root()).expect("fixture root");
    for (name, fragment, mutate) in [
        (
            "numeric-source-commit",
            "source_commit is present but is not a string",
            Box::new(|record: &mut Value| record["source_commit"] = json!(1_234_567_890_u64))
                as Box<dyn Fn(&mut Value)>,
        ),
        (
            "missing-source-commit",
            "carries no canonical source_commit string",
            Box::new(|record: &mut Value| {
                record.as_object_mut().expect("object").remove("source_commit");
            }),
        ),
        (
            "foreign-source-commit",
            "source_commit differs from the identity source field",
            Box::new(|record: &mut Value| record["source_commit"] = json!("a".repeat(40))),
        ),
        (
            "uppercase-source-commit",
            "source_commit differs from the identity source field",
            Box::new(|record: &mut Value| {
                record["source_commit"] = json!(fixture_source_commit().to_uppercase());
            }),
        ),
        (
            "numeric-state-hash",
            "state_sha256 is present but is not a string",
            Box::new(|record: &mut Value| record["state_sha256"] = json!(1)),
        ),
        (
            "string-clock",
            "clock is present but is not a positive integer",
            Box::new(|record: &mut Value| record["clock"] = json!("1024")),
        ),
        (
            "numeric-schema",
            "schema field is present but is not a string",
            Box::new(|record: &mut Value| record["schema"] = json!(1)),
        ),
    ] {
        let path = fixture_root().join(format!("mutation-{name}.json"));
        let mut record = clock_record_json(name);
        mutate(&mut record);
        fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
        let error = cli_ledger(&path).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }
}

#[test]
fn cli_accepts_the_fully_bound_ledger_record_and_preflight() {
    let record_path = fixtures::write_clock_record(&fixture_root(), "mutation-ok");
    let output = cli_ledger(&record_path).expect("fully bound record");
    let value: Value = serde_json::from_str(&output).expect("json");
    assert_eq!(value["identity_binding"]["record_source_commit"], fixture_source_commit());
    assert_eq!(value["external_source_bindings"]["included_decoder_and_model"].as_array().expect("included").len(), 2);
    let fixture = fixture("preflight-ok");
    let output = cli_preflight(&fixture.manifest_path).expect("approved plan binds");
    let value: Value = serde_json::from_str(&output).expect("json");
    assert_eq!(value["fits"], true);
    assert_eq!(value["qualification"], false);
}

#[test]
fn publication_rollback_and_enoent_messages_are_truthful() {
    let directory = fixture_root().join("truth");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("directory");

    // Completeness rollback synchronizes the parent after the removal.
    let path = directory.join("rollback.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(PublicationFault::TruncateAttached),
    )
    .expect_err("incomplete attachment must fail publication");
    assert!(error.contains("completeness"), "{error}");
    assert!(error.contains("the parent directory was synchronized"), "{error}");
    assert!(!path.exists(), "the rolled-back attachment must be gone");

    // ENOENT cleanup states absence, never a leftover that "remains".
    let path = directory.join("enoent.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(PublicationFault::CleanupAfterAttach),
    )
    .expect_err("missing temporary must fail publication conservatively");
    assert!(error.contains("already absent"), "{error}");
    assert!(!error.contains("a duplicate temporary name remains"), "{error}");
    assert_eq!(fs::read(&path).expect("attachment intact"), b"payload");
    fs::remove_file(&path).expect("cleanup");

    // The real directory-sync fault leaves the attachment and states which
    // file may lack durability, through the production message.
    let path = directory.join("sync-fault.json");
    let error = publication::publish_with_fault(
        &path,
        b"payload".as_slice(),
        Some(PublicationFault::DirectorySync),
    )
    .expect_err("directory sync must fail publication");
    assert!(error.contains("cannot be reopened"), "{error}");
    assert!(error.contains("may not be durable"), "{error}");
    assert!(error.contains("PermissionDenied"), "{error}");
    assert_eq!(fs::read(&path).expect("attachment intact"), b"payload");
    fs::remove_file(&path).expect("cleanup");
    let leftovers: Vec<_> = fs::read_dir(&directory)
        .expect("listing")
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temporary leak: {leftovers:?}");
}

/// Sanity for the CLI acceptance surface itself: the approved fixture schema
/// constant matches what the fixtures actually write.
#[test]
fn approved_plan_schema_is_the_only_admitted_plan_schema() {
    let fixture = fixture("schema-positive");
    let plan: Value =
        serde_json::from_slice(&fs::read(
            fixture.manifest_path.parent().expect("directory").join("plan.json"),
        ).expect("plan"))
        .expect("plan");
    assert_eq!(plan["schema"], json!(APPROVED_PLAN_SCHEMA));
    cli_preflight(&fixture.manifest_path).expect("approved schema binds");
}

#[test]
fn cli_ledger_refuses_forged_capture_claims_and_work_overrun() {
    let base = clock_record_json("forged");
    let identity = base["identity"].as_str().expect("identity").to_owned();
    for (name, fragment, mutate) in [
        (
            "resumable-true",
            "non-resumable captured state",
            Box::new(|record: &mut Value| record["resumable"] = json!(true))
                as Box<dyn Fn(&mut Value)>,
        ),
        (
            "qualified-true",
            "qualification=false",
            Box::new(|record: &mut Value| record["qualification"] = json!(true)),
        ),
        (
            "retained-256",
            "retained grid differs",
            Box::new(|record: &mut Value| {
                record["identity"] = json!(identity.replace("retained=512", "retained=256"));
            }),
        ),
        (
            "foreign-case",
            "frozen similarity-mms-v2 case",
            Box::new(|record: &mut Value| {
                record["identity"] =
                    json!(identity.replace(nsbu_benchmarks::CASE_SHA256, &"f".repeat(64)));
            }),
        ),
        (
            "missing-provider",
            "carries no provider field",
            Box::new(|record: &mut Value| {
                record["identity"] =
                    json!(identity.replace("provider=parallel-reduced-v2-force-w3;", ""));
            }),
        ),
        (
            "record-over-read-cap",
            "exceeds",
            Box::new(|record: &mut Value| {
                record["identity"] =
                    json!(format!("{identity};pad={}", "x".repeat((1 << 16) + 8)));
            }),
        ),
    ] {
        let path = fixture_root().join(format!("forged-{name}.json"));
        let mut record = base.clone();
        mutate(&mut record);
        fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
        let error = cli_ledger(&path).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }

    // The N512 evaluation count is enormous; a cap of ten must refuse.
    let record_path = fixtures::write_clock_record(&fixture_root(), "work-cap");
    let error = command(vec![
        "n512-ledger".into(),
        record_path.to_string_lossy().into_owned(),
        "512".into(),
        "1024".into(),
        "1024".into(),
        "32".into(),
        "10".into(),
        "281474976710656".into(),
    ])
    .expect_err("a ten-evaluation cap must refuse the N512 workload");
    assert!(error.contains("work preflight refusal"), "{error}");
}

#[test]
fn cli_ledger_refuses_an_empty_provider_field() {
    let base = clock_record_json("empty-provider");
    let identity = base["identity"].as_str().expect("identity").to_owned();
    let path = fixture_root().join("forged-empty-provider.json");
    let mut record = base.clone();
    record["identity"] =
        json!(identity.replace("provider=parallel-reduced-v2-force-w3", "provider="));
    fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
    let error = cli_ledger(&path).expect_err("empty provider");
    assert!(error.contains("carries no force provider field"), "{error}");
}

#[test]
fn ledger_normalization_refuses_each_uncovered_sample_grid_independently() {
    use nsbu_solver::domain::{Domain, Layout};
    let base = crate::ledger::AdmissionInputs {
        source: Domain::new(N, [1.0; 3], 1.0).expect("fixture domain"),
        velocity_samples: Layout::new([16; 3]).expect("fixture velocity samples"),
        pressure_samples: Layout::new([16; 3]).expect("fixture pressure samples"),
        force_samples: Layout::new([16; 3]).expect("fixture force samples"),
        workers: 1,
        backend: nsbu_solver::spectral::FftBackend::OwnedRadix,
        root_budget: 32,
        identity_len: 64,
    };
    assert!(matches!(
        crate::ledger::preflight(crate::ledger::AdmissionInputs {
            force_samples: Layout::new([4; 3]).expect("coarse force samples"),
            ..base
        }),
        Err(nsbu_solver::SolverError::InvalidDomain)
    ));
    assert!(matches!(
        crate::ledger::preflight(crate::ledger::AdmissionInputs {
            pressure_samples: Layout::new([4; 3]).expect("coarse pressure samples"),
            ..base
        }),
        Err(nsbu_solver::SolverError::InvalidDomain)
    ));
    crate::ledger::preflight(base).expect("the sound grid still binds");
}

#[test]
fn fixture_tamper_flips_a_leading_zero_digest_to_lowercase_f() {
    let path = fixture_root().join("tamper-leading-zero.json");
    fs::create_dir_all(fixture_root()).expect("fixture root");
    fs::write(&path, json!({"file_sha256": format!("0{}", "a".repeat(63))}).to_string())
        .expect("manifest write");
    let tampered_path = fixtures::tamper(&path, "leading-zero", &["file_sha256"]);
    let tampered: Value =
        serde_json::from_slice(&fs::read(tampered_path).expect("tampered")).expect("json");
    assert!(
        tampered["file_sha256"].as_str().expect("digest").starts_with('f'),
        "a leading-zero digest must be replaced with f: {tampered}"
    );
}
