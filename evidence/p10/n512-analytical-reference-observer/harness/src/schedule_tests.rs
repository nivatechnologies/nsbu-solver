#![allow(clippy::too_many_lines)]
//! Exact Astra-reference adversarial refusals for the plan admission surface:
//! required qualification/authorization flags with exact permitted values,
//! the exact named binary/watchdog/source artifact hashes (never an arbitrary
//! `*_sha256`), the evolution-bound schedule with step-multiple segments that
//! land exactly on the endpoint, and observer nodes that must each be a
//! scheduled landing point. The matching subprocess regressions live in
//! `tests/cli_adversarial.rs`.
use crate::arm_tests::{fixture, request};
use crate::{execute, Mode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn plan_variant(name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    let fixture = fixture(&format!("schedule-{name}"));
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

fn refusal(name: &str, fragment: &str, mutate: impl FnOnce(&mut Value)) {
    let path = plan_variant(name, mutate);
    let error = execute(Mode::Run, &path, request(), None).expect_err(name);
    assert!(error.contains(fragment), "{name}: {error}");
}

#[test]
fn every_qualification_and_authorization_flag_is_required_with_exact_values() {
    for key in ["qualification", "launch_authorized", "run_authorized"] {
        let remove = |plan: &mut Value| {
            plan.as_object_mut().expect("object").remove(key);
        };
        refusal(&format!("missing-{key}"), &format!("required plan flag {key:?} is missing"), remove);
        refusal(
            &format!("true-{key}"),
            &format!("plan {key} must be false"),
            |plan| plan[key] = json!(true),
        );
        refusal(
            &format!("string-{key}"),
            &format!("plan {key} must be false"),
            |plan| plan[key] = json!("false"),
        );
        refusal(
            &format!("numeric-{key}"),
            &format!("plan {key} must be false"),
            |plan| plan[key] = json!(0),
        );
    }
}

#[test]
fn only_the_named_artifact_hashes_are_admitted() {
    for key in ["binary_sha256", "watchdog_sha256", "source_sha256"] {
        refusal(
            &format!("missing-{key}"),
            &format!("artifact hash field {key:?} is missing"),
            move |plan| {
                plan.as_object_mut().expect("object").remove(key);
            },
        );
        refusal(
            &format!("short-{key}"),
            &format!("plan {key} is not 64 lowercase hexadecimal"),
            move |plan| plan[key] = json!("abc"),
        );
    }
    // An arbitrary `*_sha256` cannot substitute for the named hashes, and an
    // unapproved one is itself refused once every named hash is present.
    refusal("unapproved-artifact-hash", "unapproved artifact hash field", |plan| {
        plan["evidence_sha256"] = json!("a".repeat(64));
    });
}

#[test]
fn the_schedule_must_bind_the_evolution_and_land_exactly() {
    refusal("unbound-evolution-step", "bind the manifest evolution", |plan| {
        plan["schedule"] = json!([
            {"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 384_u64},
            {"from_inclusive": 1024_u64, "until_exclusive": 8192_u64, "step_ticks": 384_u64},
        ]);
    });
    refusal("evolution-longer-than-plan", "does not bind the manifest evolution", |plan| {
        plan["schedule"] = json!([{"from_inclusive": 0_u64, "until_exclusive": 8192_u64, "step_ticks": 512_u64}]);
    });
    refusal("segment-length-not-step-multiple", "multiple of its step", |plan| {
        plan["schedule"] = json!([
            {"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 512_u64},
            {"from_inclusive": 1024_u64, "until_exclusive": 8192_u64, "step_ticks": 3_u64},
        ]);
    });
    refusal("schedule-short-of-endpoint", "does not end exactly at the endpoint", |plan| {
        plan["schedule"] = json!([
            {"from_inclusive": 0_u64, "until_exclusive": 1024_u64, "step_ticks": 512_u64},
            {"from_inclusive": 1024_u64, "until_exclusive": 8191_u64, "step_ticks": 1_u64},
        ]);
    });
}

#[test]
fn observer_nodes_must_be_approved_by_the_schedule() {
    refusal("node-not-on-lattice", "scheduled landing point", |plan| {
        plan["observer_nodes"] = json!([0_u128, 777_u128]);
    });
    refusal("node-beyond-schedule", "scheduled landing point", |plan| {
        plan["observer_nodes"] = json!([0_u128, 1024_u128, 8191_u128]);
    });
    refusal("clock-missing-from-nodes", "not a declared observer node", |plan| {
        plan["observer_nodes"] = json!([0_u128, 512_u128]);
    });
    // A plan whose node set is exactly scheduled landing points binds.
    let path = plan_variant("approved-nodes", |plan| {
        plan["observer_nodes"] = json!([0_u128, 512_u128, 1024_u128, 4608_u128]);
    });
    execute(Mode::Preflight, &path, request(), None).expect("scheduled nodes bind");
}

#[test]
fn tampered_named_hashes_survive_consistent_rehashing() {
    // Consistently re-digested manifests (this helper always rehashes) still
    // meet the named-hash and flag requirements; the refusal is semantic.
    refusal("uppercase-named-hash", "binary_sha256 is not 64 lowercase", |plan| {
        plan["binary_sha256"] = json!(plan["binary_sha256"].as_str().expect("hash").to_uppercase());
    });
    refusal("empty-named-hash", "source_sha256\" is empty", |plan| {
        plan["source_sha256"] = json!("");
    });
}
