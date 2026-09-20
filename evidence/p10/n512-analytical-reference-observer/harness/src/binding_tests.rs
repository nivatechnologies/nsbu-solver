#![allow(clippy::too_many_lines)]
//! Binding-repair refusals: observer and ledger source/plan/record semantics,
//! publication failure uncertainty reporting, and the preserved transform and
//! 15 GiB ledger correction guards.
use crate::fixtures::{self, clock_record_json, fixture_source_commit, Fixture};
use crate::{execute, n512, provenance, Mode, Request};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-binding")
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

fn ledger_request() -> Request {
    Request {
        velocity_samples: 512,
        pressure_samples: 1024,
        force_samples: 1024,
        workers: 32,
        root_budget: 32,
        max_reference_evaluations: 1 << 40,
        cap: 1 << 48,
        velocity_floor: 1.0,
        pressure_floor: 1.0,
        backend: "rustfft-6.4.1-avx-avx2-fma".to_owned(),
    }
}

/// Write a plan variant with a re-digested manifest pointing at it.
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
fn observer_refuses_unbound_source_identities() {
    let fixture = fixture("source-binding");
    // The identity source field must equal the manifest source commit exactly.
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&fixture.manifest_path).expect("manifest")).expect("manifest");
    manifest["source_commit"] = json!("f".repeat(40));
    let diverging = fixture
        .manifest_path
        .parent()
        .expect("directory")
        .join("diverging-source.manifest.json");
    fs::write(&diverging, serde_json::to_vec_pretty(&manifest).unwrap()).expect("write");
    let error = execute(Mode::Run, &diverging, request(), None).expect_err("diverging source");
    assert!(error.contains("source field differs"), "{error}");

    run_refusal(&fixture, "plan-no-source", "no source commit", |plan| {
        let object = plan.as_object_mut().expect("plan object");
        object.remove("source_commit");
    });
    run_refusal(&fixture, "plan-foreign-source", "differs from the snapshot source", |plan| {
        plan["source_commit"] = json!("a".repeat(40));
    });
    run_refusal(&fixture, "plan-short-source", "40 lowercase", |plan| {
        plan["source_commit"] = json!("abc");
    });
    run_refusal(&fixture, "plan-foreign-profile", "plan profile differs", |plan| {
        plan["profile"] = json!("other-profile");
    });
    run_refusal(&fixture, "plan-qualified", "must be false", |plan| {
        plan["qualification"] = json!(true);
    });
    run_refusal(&fixture, "plan-launched", "must be false", |plan| {
        plan["launch_authorized"] = json!(true);
    });
    run_refusal(&fixture, "plan-no-artifact-hash", "artifact hash", |plan| {
        let object = plan.as_object_mut().expect("plan object");
        object.remove("binary_sha256");
        object.remove("watchdog_sha256");
    });
    run_refusal(&fixture, "plan-bad-artifact-hash", "64 lowercase", |plan| {
        plan["binary_sha256"] = json!("XYZ");
    });
    run_refusal(&fixture, "plan-foreign-retained", "retained layout differs", |plan| {
        plan["retained_layout"] = json!(512);
    });
    run_refusal(&fixture, "plan-foreign-force", "integration force layout differs", |plan| {
        plan["integration_force_layout"] = json!(384);
    });
    run_refusal(&fixture, "plan-missed-node", "observer node", |plan| {
        plan["observer_nodes"] = json!([0_u128]);
    });
    // Schedule, node-approval, flag-presence and named-artifact refusals live
    // in `schedule_tests` (kept out of this file for the 500-line budget).
    run_refusal(&fixture, "plan-gap-schedule", "contiguous", |plan| {
        plan["schedule"] = json!([
            {"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 512_u64},
            {"from_inclusive": 1536_u64, "until_exclusive": TARGET, "step_ticks": 512_u64},
        ]);
    });
}

#[test]
fn ledger_records_require_every_structured_binding() {
    std::fs::create_dir_all(fixture_root()).expect("fixture root");
    type LedgerCase = (&'static str, fn(&mut Value), &'static str);
    let cases: [LedgerCase; 12] = [
        (
            "non-hex-source",
            |record: &mut Value| {
                record["identity"] = json!(record["identity"].as_str().unwrap().replace(
                    &format!("source={}", fixture_source_commit()),
                    "source=not-a-commit",
                ));
            },
            "source field is not 40",
        ),
        (
            "foreign-record-commit",
            |record: &mut Value| {
                record["source_commit"] = json!("b".repeat(40));
            },
            "source_commit differs",
        ),
        (
            "schema-drift",
            |record: &mut Value| {
                record["schema"] = json!("p10-some-other-record-v9");
            },
            "identity schema field differs",
        ),
        (
            "endpoint-clock",
            |record: &mut Value| {
                record["clock"] = json!(4096_u64);
            },
            "inside the plan window",
        ),
        (
            "rest-clock",
            |record: &mut Value| {
                record["clock"] = json!(0_u64);
            },
            "clock is present but is not a positive integer",
        ),
        (
            "unsigned-artifact",
            |record: &mut Value| {
                record["state_sha256"] = json!("nope");
            },
            "state_sha256",
        ),
        (
            "zero-epoch",
            |record: &mut Value| {
                record["epoch"] = json!(0_u64);
            },
            "epoch",
        ),
        (
            "zero-coefficients",
            |record: &mut Value| {
                record["coefficient_bytes"] = json!(0_u64);
            },
            "coefficient_bytes",
        ),
        (
            "foreign-method",
            |record: &mut Value| {
                record["identity"] = json!(record["identity"]
                    .as_str()
                    .unwrap()
                    .replace("method=cox-matthews", "method=hochbruck-ostermann"));
            },
            "cox-matthews",
        ),
        (
            "unsigned-production-source",
            |record: &mut Value| {
                record["identity"] = json!(record["identity"]
                    .as_str()
                    .unwrap()
                    .split(';')
                    .map(|field| {
                        if field.starts_with("production_source=") {
                            "production_source=not-a-commit".to_owned()
                        } else {
                            field.to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(";"));
            },
            "production_source",
        ),
        (
            "missing-provider",
            |record: &mut Value| {
                record["identity"] = json!(record["identity"]
                    .as_str()
                    .unwrap()
                    .split(';')
                    .filter(|field| !field.starts_with("provider="))
                    .collect::<Vec<_>>()
                    .join(";"));
            },
            "provider",
        ),
        (
            "missing-endpoint",
            |record: &mut Value| {
                record["identity"] = json!(record["identity"]
                    .as_str()
                    .unwrap()
                    .split(';')
                    .filter(|field| !field.starts_with("endpoint="))
                    .collect::<Vec<_>>()
                    .join(";"));
            },
            "endpoint",
        ),
];
    for (name, mutate, fragment) in cases {
        let path = fixture_root().join(format!("ledger-{name}.json"));
        let mut record = clock_record_json(name);
        mutate(&mut record);
        fs::write(&path, serde_json::to_vec_pretty(&record).expect("record bytes")).expect("record write");
        let error = n512::n512_ledger(&path, ledger_request(), None).expect_err(name);
        assert!(error.contains(fragment), "{name}: {error}");
    }
}

#[test]
fn ledger_binds_the_fully_bound_fixture_record_and_echoes_identity() {
    let path = fixtures::write_clock_record(&fixture_root(), "echo");
    let output = n512::n512_ledger(&path, ledger_request(), None).expect("ledger");
    let value: Value = serde_json::from_str(&output).expect("json");
    let binding = &value["identity_binding"];
    assert_eq!(binding["source_commit"], fixture_source_commit());
    assert_eq!(binding["record_source_commit"], fixture_source_commit());
    assert_eq!(binding["method"], "cox-matthews");
    assert_eq!(binding["record_clock"], 1024);
    assert_eq!(binding["endpoint"], "4096");
    assert_eq!(binding["case_sha256"], nsbu_benchmarks::CASE_SHA256);
    assert!(value["qualification"].is_boolean());
    assert_eq!(value["accepted_windows"], 0);
}

#[test]
fn transform_count_is_exactly_the_reviewed_fifty_sequence() {
    let fixture = fixture("transforms");
    let output = execute(Mode::Run, &fixture.manifest_path, request(), None).expect("run");
    let value: Value = serde_json::from_str(&output).expect("json");
    let executed = value["observations"]["precision_identity"]["executed_scalar_transforms"]
        .as_u64()
        .expect("executed");
    assert_eq!(executed, 50, "3+9+27+6 velocity + 1+3 pressure + 1 gauge witness");
    assert_eq!(
        executed,
        value["work"]["observer_scalar_transforms"].as_u64().expect("planned"),
        "planned and executed scalar transforms must agree exactly"
    );
    let per_quantity: Vec<u64> = value["observations"]["quantities"]
        .as_array()
        .expect("quantities")
        .iter()
        .map(|quantity| quantity["actual_scalar_transforms"].as_u64().expect("transforms"))
        .collect();
    assert_eq!(per_quantity, vec![3, 9, 27, 6, 1, 3]);
}

#[test]
fn fifteen_gib_ledger_correction_term_is_pinned() {
    let velocity = 512_usize.pow(3);
    let pressure = 1024_usize.pow(3);
    assert_eq!(
        n512::expected_comparison_array_bytes(velocity, pressure)
            - (3 * velocity + 2 * pressure) * std::mem::size_of::<f64>(),
        16_106_127_360,
        "the corrected live-array term must exceed the old allowance by exactly 15 GiB"
    );
}

#[test]
fn manifest_envelope_refuses_every_disagreement_between_the_two_bounded_reads() {
    let fixture = fixture("envelope-reads");
    let text = fs::read_to_string(&fixture.manifest_path).expect("manifest text");
    let sound: crate::model::Manifest = serde_json::from_str(&text).expect("decoded manifest");
    let raw_of = |text: &str| -> Value { serde_json::from_str(text).expect("raw manifest") };
    let refuse = |raw: &Value, manifest: &crate::model::Manifest, fragment: &str, label: &str| {
        let error = provenance::bind_manifest_envelope(raw, manifest)
            .expect_err(label);
        assert!(error.contains(fragment), "{label}: {error}");
    };

    let mut schema = raw_of(&text);
    schema["schema"] = json!("p10-tampered-schema-v0");
    refuse(&schema, &sound, "manifest schema", "schema disagreement");

    for key in [
        "identity", "plan_sha256", "file_sha256", "coefficient_sha256",
        "source_commit", "backend", "execution",
    ] {
        let mut raw = raw_of(&text);
        raw[key] = json!("f".repeat(64));
        refuse(&raw, &sound, "differs from the decoded snapshot", key);
    }

    let mut evolution = raw_of(&text);
    evolution["evolution"]["case_sha256"] = json!("f".repeat(64));
    refuse(&evolution, &sound, "evolution case differs", "evolution case");
    evolution["evolution"]
        .as_object_mut()
        .expect("evolution object")
        .remove("case_sha256");
    refuse(&evolution, &sound, "carries no case_sha256", "evolution field");

    // Identical wrong-typed values in both reads still meet the hex and size
    // floors: equality alone never admits arbitrary bytes.
    let mut unhex = raw_of(&text);
    unhex["source_commit"] = json!("z".repeat(40));
    let unhex_text = serde_json::to_string(&unhex).expect("unhex text");
    let unhex_manifest: crate::model::Manifest =
        serde_json::from_str(&unhex_text).expect("unhex manifest");
    refuse(&unhex, &unhex_manifest, "is not 40 lowercase hex", "commit hex");

    let mut unhash = raw_of(&text);
    unhash["plan_sha256"] = json!("z".repeat(64));
    let unhash_text = serde_json::to_string(&unhash).expect("unhash text");
    let unhash_manifest: crate::model::Manifest =
        serde_json::from_str(&unhash_text).expect("unhash manifest");
    refuse(&unhash, &unhash_manifest, "plan_sha256 is not 64 lowercase", "hash hex");

    let mut huge = raw_of(&text);
    huge["identity"] = json!(format!("{};pad={}", sound.identity, "x".repeat((1 << 16) + 8)));
    let huge_text = serde_json::to_string(&huge).expect("huge text");
    let huge_manifest: crate::model::Manifest =
        serde_json::from_str(&huge_text).expect("huge manifest");
    refuse(&huge, &huge_manifest, "identity exceeds the bounded length", "identity bound");
}
