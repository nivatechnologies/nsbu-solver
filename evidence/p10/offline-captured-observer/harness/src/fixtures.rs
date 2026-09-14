#![allow(dead_code)]
//! Test fixtures: build small reviewed-format snapshot manifests without weakening
//! any decoder admission; every fixture uses an already admitted decoder profile.
use nsbu_solver::{domain::Layout, Complex64};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Fixture {
    pub manifest_path: PathBuf,
    pub coefficients: [Vec<Complex64>; 3],
    pub coefficient_sha256: String,
    pub file_sha256: String,
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
) -> Result<Fixture, String> {
    let directory = root.join(name);
    fs::create_dir_all(&directory).map_err(|error| format!("{error:?}"))?;
    let layout = Layout::new(dimensions).map_err(|error| format!("{error:?}"))?;
    for component in coefficients {
        if component.len() != layout.half_len() {
            return Err("fixture coefficient shape mismatch".into());
        }
    }

    let plan_bytes = json!({"fixture-plan": name}).to_string().into_bytes();
    fs::write(directory.join("plan.json"), &plan_bytes).map_err(|error| format!("{error:?}"))?;
    let plan_sha256 = format!("{:x}", Sha256::digest(&plan_bytes));

    let identity = format!("p10-offline-observer-fixture;name={name};profile=owned-radix-fixture");
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
            let bytes = [
                value.re.to_bits().to_le_bytes(),
                value.im.to_bits().to_le_bytes(),
            ];
            for half in bytes {
                coefficient_hash.update(half);
                snapshot.extend(half);
            }
        }
    }
    let coefficient_digest = coefficient_hash.finalize();
    snapshot.extend(coefficient_digest.as_slice());
    let file_sha256 = format!("{:x}", Sha256::digest(&snapshot));
    fs::write(directory.join("snapshot.bin"), &snapshot).map_err(|error| format!("{error:?}"))?;

    let manifest = json!({
        "schema": "p10-snapshot-comparison-input-v1",
        "comparison_kind": comparison_kind,
        "snapshot": "snapshot.bin",
        "plan": "plan.json",
        "identity": identity,
        "source_commit": hex40("offline-observer-fixture-commit"),
        "plan_sha256": plan_sha256,
        "coefficient_sha256": format!("{coefficient_digest:x}"),
        "file_sha256": file_sha256,
        "backend": "owned-radix",
        "execution": "offline-observer-tests",
        "dimensions": dimensions,
        "evolution": {
            "case_sha256": nsbu_benchmarks::CASE_SHA256,
            "quantum_exponent": -20_i64,
            "clock_target": target,
            "comparison_endpoint": elapsed,
            "lengths": [1.0, 1.0, 1.0],
            "viscosity": 1.0,
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
    });
    let manifest_path = directory.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok(Fixture {
        manifest_path,
        coefficients: coefficients.clone(),
        coefficient_sha256: format!("{coefficient_digest:x}"),
        file_sha256,
    })
}

pub fn tamper(manifest_path: &Path, name: &str, field: &str) -> Result<PathBuf, String> {
    let bytes = fs::read(manifest_path).map_err(|error| format!("{error:?}"))?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let current = value[field]
        .as_str()
        .ok_or("fixture field is not a string")?
        .to_string();
    let replaced = if current.starts_with('0') { 'f' } else { '0' };
    value[field] = serde_json::Value::String(format!("{replaced}{}", &current[1..]));
    let path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    Ok(path)
}

pub fn coefficient_hash(coefficients: [&[Complex64]; 3]) -> String {
    let mut hash = Sha256::new();
    for component in coefficients {
        for value in component {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

fn hex40(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))[..40].to_owned()
}

fn hex64(seed: &str) -> String {
    format!("{:x}", Sha256::digest(seed.as_bytes()))
}
