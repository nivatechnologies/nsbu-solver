#![allow(dead_code)]
//! Test fixtures: build small reviewed-format snapshot manifests without weakening
//! any decoder admission. Identities carry the exact `case=`, `retained=` and
//! `provider=` fields the observer binds; every fixture uses an already admitted
//! decoder profile (cox-matthews with an M512 integration-force grid). Plans
//! carry the approved named artifact hashes, the required qualification flags
//! and an evolution-bound schedule whose segments land exactly on their bounds.
//! I/O failures are test-infrastructure failures: fixture helpers `expect`
//! through them, so no unexercised error arm inflates fixture complexity.
use nsbu_solver::{domain::Layout, Complex64};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Fixture {
    pub manifest_path: PathBuf,
    pub coefficients: [Vec<Complex64>; 3],
    pub coefficient_sha256: String,
    pub file_sha256: String,
    pub source_commit: String,
    pub identity: String,
}

pub fn fixture_source_commit() -> String {
    hex40("analytical-reference-observer-fixture-commit")
}

/// Two divergence-free half-spectrum modes below every Nyquist plane; all other
/// retained coefficients are exact zero, so the k=0 Hermitian pairing is exact.
pub fn synthetic_state(layout: Layout) -> [Vec<Complex64>; 3] {
    let zero = Complex64::new(0.0, 0.0);
    let mut coefficients = [
        vec![zero; layout.half_len()],
        vec![zero; layout.half_len()],
        vec![zero; layout.half_len()],
    ];
    for (mode, amplitude, direction) in [
        (
            [1_usize, 2, 3],
            Complex64::new(0.25, 0.125),
            [2.0, -1.0, 0.0],
        ),
        (
            [2_usize, 1, 1],
            Complex64::new(-0.125, 0.0625),
            [1.0, -2.0, 0.0],
        ),
    ] {
        let index = layout.index(mode).expect("fixture mode inside the band");
        for (axis, &scale) in direction.iter().enumerate() {
            coefficients[axis][index] = amplitude * scale;
        }
    }
    coefficients
}

#[allow(clippy::too_many_arguments)]
pub fn write_fixture(
    root: &Path,
    name: &str,
    coefficients: &[Vec<Complex64>; 3],
    comparison_kind: &str,
    method: &str,
    integration_force_dimensions: [usize; 3],
    elapsed: u128,
    target: u128,
    dimensions: [usize; 3],
    lengths: [f64; 3],
    viscosity: f64,
) -> Fixture {
    let directory = root.join(name);
    fs::create_dir_all(&directory).expect("fixture directory");
    let source_commit = fixture_source_commit();
    let identity = fixture_identity(name, &source_commit, dimensions, method, target);
    let (snapshot, coefficient_sha256) = snapshot_bytes(&identity, elapsed, target, coefficients);
    let file_sha256 = hex64_bytes(&snapshot);
    let plan = fixture_plan(&source_commit, name, dimensions, integration_force_dimensions, elapsed, target);
    write_json(&directory.join("plan.json"), &plan);
    let manifest = fixture_manifest(
        comparison_kind,
        &identity,
        &source_commit,
        &hex64_bytes(&serde_json::to_vec_pretty(&plan).expect("plan bytes")),
        &coefficient_sha256,
        &file_sha256,
        dimensions,
        integration_force_dimensions,
        method,
        lengths,
        viscosity,
        elapsed,
        target,
    );
    fs::write(directory.join("snapshot.bin"), &snapshot).expect("fixture snapshot write");
    let manifest_path = directory.join("manifest.json");
    write_json(&manifest_path, &manifest);
    Fixture {
        manifest_path,
        coefficients: coefficients.to_vec().try_into().expect("three components"),
        coefficient_sha256,
        file_sha256,
        source_commit,
        identity,
    }
}

/// The approved-shape plan: closed schema, the exact named binary/watchdog/
/// source artifact hashes, required false flags, and an evolution-bound
/// schedule (the fixture evolution is one segment `[0, elapsed)`) extended to
/// exactly the endpoint by step-multiple segments.
#[allow(clippy::too_many_arguments)]
fn fixture_plan(
    source_commit: &str,
    name: &str,
    dimensions: [usize; 3],
    integration_force_dimensions: [usize; 3],
    elapsed: u128,
    target: u128,
) -> Value {
    let step = elapsed / 2;
    let mut schedule = vec![json!({
        "from_inclusive": 0_u128,
        "until_exclusive": elapsed,
        "step_ticks": step,
    })];
    if target > elapsed {
        schedule.push(json!({
            "from_inclusive": elapsed,
            "until_exclusive": target,
            "step_ticks": step,
        }));
    }
    json!({
        "schema": crate::plan::APPROVED_PLAN_SCHEMA,
        "source_commit": source_commit,
        "profile": "owned-radix-fixture",
        "retained_layout": dimensions[0],
        "integration_force_layout": integration_force_dimensions[0],
        "endpoint_ticks": target,
        "schedule": schedule,
        "observer_nodes": [0_u128, elapsed],
        "binary_sha256": hex64(&format!("fixture-binary-{name}")),
        "watchdog_sha256": hex64("fixture-watchdog"),
        "source_sha256": hex64(&format!("fixture-source-{name}")),
        "qualification": false,
        "launch_authorized": false,
        "run_authorized": false,
    })
}

fn fixture_identity(
    name: &str,
    source_commit: &str,
    dimensions: [usize; 3],
    method: &str,
    target: u128,
) -> String {
    format!(
        "p10-analytical-reference-fixture;name={name};source={source_commit};\
         profile=owned-radix-fixture;retained={};case={};method={method};\
         endpoint={target};schema=p10-analytical-reference-fixture-state-v1;\
         provider=fixture-parallel-reduced-v2",
        dimensions[0],
        nsbu_benchmarks::CASE_SHA256
    )
}

#[allow(clippy::too_many_arguments)]
fn fixture_manifest(
    comparison_kind: &str,
    identity: &str,
    source_commit: &str,
    plan_sha256: &str,
    coefficient_sha256: &str,
    file_sha256: &str,
    dimensions: [usize; 3],
    integration_force_dimensions: [usize; 3],
    method: &str,
    lengths: [f64; 3],
    viscosity: f64,
    elapsed: u128,
    target: u128,
) -> Value {
    json!({
        "schema": "p10-snapshot-comparison-input-v1",
        "comparison_kind": comparison_kind,
        "snapshot": "snapshot.bin",
        "plan": "plan.json",
        "identity": identity,
        "source_commit": source_commit,
        "plan_sha256": plan_sha256,
        "coefficient_sha256": coefficient_sha256,
        "file_sha256": file_sha256,
        "backend": "owned-radix",
        "execution": "analytical-reference-observer-tests",
        "dimensions": dimensions,
        "evolution": {
            "case_sha256": nsbu_benchmarks::CASE_SHA256,
            "quantum_exponent": -20_i64,
            "clock_target": target,
            "comparison_endpoint": elapsed,
            "lengths": lengths,
            "viscosity": viscosity,
            "method": method,
            "integration_force_dimensions": integration_force_dimensions,
            "schedule": [{
                "from_inclusive": 0_u128,
                "until_exclusive": elapsed,
                "step_ticks": elapsed / 2,
            }],
            "absolute_tolerances": [1e-5, 1e-4],
            "relative_tolerances": [1e-5, 1e-5],
        },
        "elapsed": elapsed,
        "target": target,
        "epoch": 0_u128,
        "accepted_steps": 8_u128,
    })
}

pub(crate) fn snapshot_bytes(
    identity: &str,
    elapsed: u128,
    target: u128,
    coefficients: &[Vec<Complex64>; 3],
) -> (Vec<u8>, String) {
    let mut snapshot = Vec::new();
    snapshot.extend(b"P10AVXSNAP1\0");
    snapshot.extend((identity.len() as u64).to_le_bytes());
    snapshot.extend(identity.as_bytes());
    for value in [elapsed, target, 0_u128, 8_u128] {
        snapshot.extend(value.to_le_bytes());
    }
    let mut coefficient_hash = Sha256::new();
    for component in coefficients {
        for value in component {
            let halves = [
                value.re.to_bits().to_le_bytes(),
                value.im.to_bits().to_le_bytes(),
            ];
            for half in halves {
                coefficient_hash.update(half);
                snapshot.extend(half);
            }
        }
    }
    let coefficient_digest = coefficient_hash.finalize();
    snapshot.extend(coefficient_digest.as_slice());
    (snapshot, format!("{coefficient_digest:x}"))
}

fn write_json(path: &Path, value: &Value) {
    let bytes = serde_json::to_vec_pretty(value).expect("fixture json");
    fs::write(path, bytes).expect("fixture write");
}

fn hex64_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Replace one string leaf of a manifest with a tampered hex digit, writing a
/// sibling manifest. Tamper paths assert fixture infrastructure, not product
/// behavior, so they `expect` rather than carry uncovered error arms.
pub fn tamper(manifest_path: &Path, name: &str, key_path: &[&str]) -> PathBuf {
    let bytes = fs::read(manifest_path).expect("fixture manifest read");
    let mut value: Value = serde_json::from_slice(&bytes).expect("fixture manifest json");
    let mut cursor = &mut value;
    for key in &key_path[..key_path.len() - 1] {
        cursor = cursor.get_mut(key).expect("fixture tamper path");
    }
    let leaf = *key_path.last().expect("non-empty fixture tamper path");
    let current = cursor[leaf].as_str().expect("fixture string field").to_string();
    let replaced = if current.starts_with('0') { 'f' } else { '0' };
    cursor[leaf] = Value::String(format!("{replaced}{}", &current[1..]));
    let path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{name}.json"));
    write_json(&path, &value);
    path
}

fn hex40(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))[..40].to_owned()
}

fn hex64(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))
}

/// Arithmetic-only ledger clock record in the fully bound structured shape.
pub fn write_clock_record(root: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(root).expect("fixture record root");
    let path = root.join(format!("{name}.json"));
    write_json(&path, &clock_record_json(name));
    path
}

/// The structured clock record every N512 ledger admission must bind: schema,
/// source, profile, method, endpoint-bounded clock, epoch/step counts and the
/// state artifact hash, alongside the frozen case and retained 512 grid.
pub fn clock_record_json(name: &str) -> Value {
    json!({
        "schema": "p10-analytical-reference-fixture-record-v1",
        "identity": format!(
            "source={};case={};retained=512;profile=n512-{name};\
             provider=parallel-reduced-v2-force-w3;method=cox-matthews;\
             endpoint=4096;schema=p10-analytical-reference-fixture-record-v1;\
             production_source={};test_source={}",
            fixture_source_commit(),
            nsbu_benchmarks::CASE_SHA256,
            hex40("fixture-production-source"),
            hex40("fixture-test-source"),
        ),
        "source_commit": fixture_source_commit(),
        "resumable": false,
        "qualification": false,
        "clock": 1024_u64,
        "epoch": 8_u64,
        "accepted_steps": 8_u64,
        "coefficient_bytes": 3_233_808_384_u64,
        "state_sha256": hex64(&format!("fixture-state-{name}")),
        "observation_status": "CapturedActualState",
    })
}
