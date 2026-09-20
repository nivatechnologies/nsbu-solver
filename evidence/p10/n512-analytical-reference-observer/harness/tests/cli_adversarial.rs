//! Real subprocess CLI adversarial regressions: the compiled observer binary
//! is launched for every exact Astra-reference mutation. Each refusal must
//! exit non-zero with the exact semantic message on stderr, and the accepted
//! paths must exit zero with outputs carrying the compiled-byte and aggregate
//! digests recomputed independently from the repository sources.

use nsbu_solver::domain::Layout;
use nsbu_solver::Complex64;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, path::PathBuf, process::Command};

const N: [usize; 3] = [8; 3];
const ELAPSED: u128 = 1024;
const TARGET: u128 = 8192;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_p10-n512-analytical-reference-observer")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(binary()).args(args).output().expect("spawn observer");
    (
        output.status.code().expect("exit code"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn preflight_args(manifest: &Path) -> Vec<String> {
    [
        "preflight".into(),
        manifest.to_string_lossy().into_owned(),
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
    ]
    .to_vec()
}

fn ledger_args(record: &Path) -> Vec<String> {
    [
        "n512-ledger".into(),
        record.to_string_lossy().into_owned(),
        "512".into(),
        "1024".into(),
        "1024".into(),
        "32".into(),
        "1099511627776".into(),
        "281474976710656".into(),
    ]
    .to_vec()
}

fn hex40(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))[..40].to_owned()
}

fn hex64(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))
}

fn source_commit() -> String {
    hex40("analytical-reference-observer-fixture-commit")
}

/// Build a fully valid reviewed-format fixture (plan + snapshot + manifest)
/// beside `root`, then apply `mutate` to the plan and re-digest the manifest
/// consistently: an attacker who re-hashes everything must still be refused.
fn mutated_manifest(root: &Path, name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    fs::create_dir_all(root).expect("fixture root");
    let directory = root.join(name);
    fs::create_dir_all(&directory).expect("fixture directory");
    let commit = source_commit();
    let identity = format!(
        "p10-analytical-reference-fixture;name={name};source={commit};\
         profile=owned-radix-fixture;retained=8;case={};method=cox-matthews;\
         endpoint={TARGET};schema=p10-analytical-reference-fixture-state-v1;\
         provider=fixture-parallel-reduced-v2",
        nsbu_benchmarks::CASE_SHA256
    );
    let mut plan = json!({
        "schema": "p10-n512-analytical-reference-plan-v1",
        "source_commit": commit,
        "profile": "owned-radix-fixture",
        "retained_layout": N[0],
        "integration_force_layout": 512,
        "endpoint_ticks": TARGET,
        "schedule": [
            {"from_inclusive": 0_u128, "until_exclusive": ELAPSED, "step_ticks": 512_u64},
            {"from_inclusive": ELAPSED, "until_exclusive": TARGET, "step_ticks": 512_u64},
        ],
        "observer_nodes": [0_u128, ELAPSED],
        "binary_sha256": hex64(&format!("fixture-binary-{name}")),
        "watchdog_sha256": hex64("fixture-watchdog"),
        "source_sha256": hex64(&format!("fixture-source-{name}")),
        "qualification": false,
        "launch_authorized": false,
        "run_authorized": false,
    });
    mutate(&mut plan);
    let plan_bytes = serde_json::to_vec_pretty(&plan).expect("plan bytes");
    fs::write(directory.join("plan.json"), &plan_bytes).expect("plan write");

    let (snapshot, coefficient_sha256) = snapshot_bytes(&identity);
    fs::write(directory.join("snapshot.bin"), &snapshot).expect("snapshot write");
    let manifest = json!({
        "schema": "p10-snapshot-comparison-input-v1",
        "comparison_kind": "MATCHED_SPATIAL",
        "snapshot": "snapshot.bin",
        "plan": "plan.json",
        "identity": identity,
        "source_commit": commit,
        "plan_sha256": format!("{:x}", Sha256::digest(&plan_bytes)),
        "coefficient_sha256": coefficient_sha256,
        "file_sha256": format!("{:x}", Sha256::digest(&snapshot)),
        "backend": "owned-radix",
        "execution": "analytical-reference-observer-cli-tests",
        "dimensions": N,
        "evolution": {
            "case_sha256": nsbu_benchmarks::CASE_SHA256,
            "quantum_exponent": -20_i64,
            "clock_target": TARGET,
            "comparison_endpoint": ELAPSED,
            "lengths": [1.0, 1.0, 1.0],
            "viscosity": 1.0,
            "method": "cox-matthews",
            "integration_force_dimensions": [512, 512, 512],
            "schedule": [{
                "from_inclusive": 0_u128, "until_exclusive": ELAPSED, "step_ticks": 512_u64,
            }],
            "absolute_tolerances": [1e-5, 1e-4],
            "relative_tolerances": [1e-5, 1e-5],
        },
        "elapsed": ELAPSED,
        "target": TARGET,
        "epoch": 0_u128,
        "accepted_steps": 8_u128,
    });
    let manifest_path = directory.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).expect("manifest")).expect("write");
    manifest_path
}

fn snapshot_bytes(identity: &str) -> (Vec<u8>, String) {
    let layout = Layout::new(N).expect("layout");
    let zero = Complex64::new(0.0, 0.0);
    let mut coefficients = [
        vec![zero; layout.half_len()],
        vec![zero; layout.half_len()],
        vec![zero; layout.half_len()],
    ];
    for (mode, amplitude, direction) in [
        ([1_usize, 2, 3], Complex64::new(0.25, 0.125), [2.0, -1.0, 0.0]),
        ([2_usize, 1, 1], Complex64::new(-0.125, 0.0625), [1.0, -2.0, 0.0]),
    ] {
        let index = layout.index(mode).expect("mode");
        for (axis, &scale) in direction.iter().enumerate() {
            coefficients[axis][index] = amplitude * scale;
        }
    }
    let mut snapshot: Vec<u8> = Vec::new();
    snapshot.extend(b"P10AVXSNAP1\0");
    snapshot.extend((identity.len() as u64).to_le_bytes());
    snapshot.extend(identity.as_bytes());
    for value in [ELAPSED, TARGET, 0_u128, 8_u128] {
        snapshot.extend(value.to_le_bytes());
    }
    let mut hash = Sha256::new();
    for component in &coefficients {
        for value in component {
            for half in [
                value.re.to_bits().to_le_bytes(),
                value.im.to_bits().to_le_bytes(),
            ] {
                hash.update(half);
                snapshot.extend(half);
            }
        }
    }
    let digest = hash.finalize();
    snapshot.extend(digest.as_slice());
    (snapshot, format!("{digest:x}"))
}

fn ledger_record(root: &Path, name: &str, mutate: impl FnOnce(&mut Value)) -> PathBuf {
    fs::create_dir_all(root).expect("fixture root");
    let mut record = json!({
        "schema": "p10-analytical-reference-fixture-record-v1",
        "identity": format!(
            "source={};case={};retained=512;profile=n512-{name};\
             provider=parallel-reduced-v2-force-w3;method=cox-matthews;\
             endpoint=4096;schema=p10-analytical-reference-fixture-record-v1;\
             production_source={};test_source={}",
            source_commit(),
            nsbu_benchmarks::CASE_SHA256,
            hex40("fixture-production-source"),
            hex40("fixture-test-source"),
        ),
        "source_commit": source_commit(),
        "resumable": false,
        "qualification": false,
        "clock": 1024_u64,
        "epoch": 8_u64,
        "accepted_steps": 8_u64,
        "coefficient_bytes": 3_233_808_384_u64,
        "state_sha256": hex64(&format!("fixture-state-{name}")),
        "observation_status": "CapturedActualState",
    });
    mutate(&mut record);
    let path = root.join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&record).expect("record")).expect("write");
    path
}

fn refuse_plan(name: &str, fragment: &str, mutate: impl FnOnce(&mut Value)) {
    let root = std::env::temp_dir().join("p10-observer-cli-adversarial");
    let manifest = mutated_manifest(&root, name, mutate);
    let args = preflight_args(&manifest);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, _stdout, stderr) = run(&args);
    assert_eq!(code, 1, "{name} must exit non-zero");
    assert!(
        stderr.contains("analytical-reference-observer-refusal") && stderr.contains(fragment),
        "{name}: {stderr}"
    );
}

#[test]
fn subprocess_refuses_exact_flag_and_artifact_mutations() {
    for (name, fragment, mutate) in [
        (
            "missing-qualification",
            "required plan flag \"qualification\" is missing",
            Box::new(|plan: &mut Value| {
                plan.as_object_mut().expect("object").remove("qualification");
            }) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "launched-authorized",
            "plan launch_authorized must be false",
            Box::new(|plan: &mut Value| plan["launch_authorized"] = json!(true)),
        ),
        (
            "run-authorized-string",
            "plan run_authorized must be false",
            Box::new(|plan: &mut Value| plan["run_authorized"] = json!("false")),
        ),
        (
            "missing-watchdog-hash",
            "artifact hash field \"watchdog_sha256\" is missing",
            Box::new(|plan: &mut Value| {
                plan.as_object_mut().expect("object").remove("watchdog_sha256");
            }),
        ),
        (
            "missing-source-hash",
            "artifact hash field \"source_sha256\" is missing",
            Box::new(|plan: &mut Value| {
                plan.as_object_mut().expect("object").remove("source_sha256");
            }),
        ),
        (
            "unapproved-hash",
            "unapproved artifact hash field \"evidence_sha256\"",
            Box::new(|plan: &mut Value| plan["evidence_sha256"] = json!("a".repeat(64))),
        ),
    ] {
        refuse_plan(name, fragment, mutate);
    }
}

#[test]
fn subprocess_refuses_exact_schedule_and_node_mutations() {
    for (name, fragment, mutate) in [
        (
            "evolution-unbound-schedule",
            "does not bind the manifest evolution schedule",
            Box::new(|plan: &mut Value| {
                plan["schedule"] = json!([
                    {"from_inclusive": 0_u64, "until_exclusive": ELAPSED, "step_ticks": 384_u64},
                    {"from_inclusive": ELAPSED, "until_exclusive": TARGET, "step_ticks": 384_u64},
                ]);
            }) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "segment-not-step-multiple",
            "not a multiple of its step",
            Box::new(|plan: &mut Value| {
                plan["schedule"] = json!([
                    {"from_inclusive": 0_u64, "until_exclusive": ELAPSED, "step_ticks": 512_u64},
                    {"from_inclusive": ELAPSED, "until_exclusive": TARGET, "step_ticks": 3_u64},
                ]);
            }),
        ),
        (
            "schedule-short-endpoint",
            "does not end exactly at the endpoint",
            Box::new(|plan: &mut Value| {
                plan["schedule"] = json!([
                    {"from_inclusive": 0_u64, "until_exclusive": ELAPSED, "step_ticks": 512_u64},
                    {"from_inclusive": ELAPSED, "until_exclusive": TARGET - 1, "step_ticks": 1_u64},
                ]);
            }),
        ),
        (
            "node-off-lattice",
            "observer node 777 is not a scheduled landing point",
            Box::new(|plan: &mut Value| plan["observer_nodes"] = json!([0_u128, 777_u128])),
        ),
    ] {
        refuse_plan(name, fragment, mutate);
    }
}

#[test]
fn subprocess_ledger_refuses_missing_state_hash_and_accepts_the_bound_record() {
    let root = std::env::temp_dir().join("p10-observer-cli-adversarial");
    let record = ledger_record(&root, "missing-state", |record| {
        record.as_object_mut().expect("object").remove("state_sha256");
    });
    let args = ledger_args(&record);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, _stdout, stderr) = run(&args);
    assert_eq!(code, 1);
    assert!(stderr.contains("carries no state_sha256 artifact hash"), "{stderr}");

    let record = ledger_record(&root, "valid", |_record| {});
    let args = ledger_args(&record);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0, "stderr: {stderr}");
    let value: Value = serde_json::from_str(&stdout).expect("ledger json");
    assert_eq!(value["qualification"], false);
    let bindings = &value["external_source_bindings"];
    assert_eq!(
        bindings["path_dependency_aggregate_digests"]["nsbu-benchmarks"]
            .as_str()
            .expect("benchmarks aggregate"),
        disk_aggregate("nsbu-benchmarks"),
        "the published aggregate must be reproducible from the repository sources"
    );
    assert_eq!(
        bindings["path_dependency_aggregate_digests"]["nsbu-solver"]
            .as_str()
            .expect("solver aggregate"),
        disk_aggregate("nsbu-solver"),
        "the published aggregate must be reproducible from the repository sources"
    );
    let inventory = fs::read(inventory_path()).expect("inventory");
    assert_eq!(bindings["sealed_inventory_sha256"], format!("{:x}", Sha256::digest(&inventory)));
    for role in ["decoder", "model"] {
        let entry = bindings["included_decoder_and_model"]
            .as_array()
            .expect("included")
            .iter()
            .find(|source| source["role"].as_str() == Some(role))
            .expect("role");
        let repository_path = entry["repository_path"].as_str().expect("path");
        let on_disk = fs::read(repo_root().join(repository_path)).expect("included source");
        assert_eq!(entry["sha256"], format!("{:x}", Sha256::digest(&on_disk)));
    }
}

fn inventory_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../source-inventory.json")
}

/// Recompute one dependency crate's aggregate from the repository sources:
/// SHA-256 over sorted `repository_path  sha256\n` lines (Cargo.lock excluded),
/// exactly the sealed-inventory rule.
fn disk_aggregate(crate_name: &str) -> String {
    let crate_root = repo_root().join("crates").join(crate_name);
    let mut lines: Vec<String> = Vec::new();
    let mut stack = vec![crate_root];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("directory listing") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name != "Cargo.lock") {
                let relative = path
                    .strip_prefix(repo_root())
                    .expect("repository-relative")
                    .to_string_lossy()
                    .into_owned();
                let digest = format!("{:x}", Sha256::digest(fs::read(&path).expect("source")));
                lines.push(format!("{relative}  {digest}\n"));
            }
        }
    }
    lines.sort();
    let mut aggregate = Sha256::new();
    for line in lines {
        aggregate.update(line.as_bytes());
    }
    format!("{:x}", aggregate.finalize())
}

#[test]
fn subprocess_preflight_accepts_the_fully_bound_plan() {
    let root = std::env::temp_dir().join("p10-observer-cli-adversarial");
    let manifest = mutated_manifest(&root, "accepted", |_plan| {});
    let args = preflight_args(&manifest);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0, "stderr: {stderr}");
    let value: Value = serde_json::from_str(&stdout).expect("preflight json");
    assert_eq!(value["fits"], true);
    assert_eq!(value["qualification"], false);
    assert_eq!(value["accepted_windows"], 0);
    assert_eq!(
        value["external_source_bindings"]["path_dependency_aggregate_digests"]
            .as_object()
            .expect("aggregates")
            .len(),
        2
    );
}
