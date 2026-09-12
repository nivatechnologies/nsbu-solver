use super::{compare, decode};
use crate::model::{ClockHeader, Evolution, Manifest, ScheduleSegment};
use nsbu_solver::{diagnostics::comparison::ComparisonPlan, domain::Layout, Complex64};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "p10-snapshot-comparison-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn manifest(path: PathBuf, n: usize, identity: &str) -> Manifest {
    let plan = path.with_file_name(format!("plan-{identity}.json"));
    fs::write(&plan, b"{\"reviewed\":true}\n").unwrap();
    Manifest {
        schema: "p10-snapshot-comparison-input-v1".into(),
        snapshot: path,
        plan,
        identity: identity.into(),
        source_commit: "a".repeat(40),
        plan_sha256: format!("{:x}", Sha256::digest(b"{\"reviewed\":true}\n")),
        coefficient_sha256: String::new(),
        file_sha256: String::new(),
        backend: "fixture-backend".into(),
        execution: format!("fixture-{n}"),
        dimensions: [n; 3],
        evolution: Evolution {
            case_sha256: "c".repeat(64),
            quantum_exponent: -20,
            clock_target: 64,
            comparison_endpoint: 64,
            lengths: [1.0; 3],
            viscosity: 0.01,
            method: "cox-matthews".into(),
            integration_force_dimensions: [384; 3],
            schedule: vec![ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 64,
                step_ticks: 32,
            }],
            absolute_tolerances: [1e-5, 1e-4],
            relative_tolerances: [1e-5, 1e-5],
        },
        elapsed: 64,
        target: 64,
        epoch: 1,
        accepted_steps: 2,
    }
}

fn fields(layout: Layout, scale: f64) -> [Vec<Complex64>; 3] {
    let mut result = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    for (axis, component) in result.iter_mut().enumerate() {
        component[0] = Complex64::new(scale * (axis + 1) as f64, 0.0);
        let a = layout.index([1, 0, 0]).unwrap();
        let b = layout.index([layout.dimensions()[0] - 1, 0, 0]).unwrap();
        component[a] = Complex64::new(scale, scale * 0.25);
        component[b] = component[a].conj();
    }
    result
}

fn encode(manifest: &Manifest, fields: &[Vec<Complex64>; 3]) -> Vec<u8> {
    let mut bytes = b"P10AVXSNAP1\0".to_vec();
    bytes.extend_from_slice(&(manifest.identity.len() as u64).to_le_bytes());
    bytes.extend_from_slice(manifest.identity.as_bytes());
    for value in [
        manifest.elapsed,
        manifest.target,
        manifest.epoch,
        manifest.accepted_steps,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let payload_start = bytes.len();
    for component in fields {
        for value in component {
            bytes.extend_from_slice(&value.re.to_bits().to_le_bytes());
            bytes.extend_from_slice(&value.im.to_bits().to_le_bytes());
        }
    }
    bytes.extend_from_slice(&Sha256::digest(&bytes[payload_start..]));
    bytes
}

fn write(manifest: &mut Manifest, fields: &[Vec<Complex64>; 3]) {
    let bytes = encode(manifest, fields);
    let payload = 12 + 8 + manifest.identity.len() + 4 * 16;
    manifest.coefficient_sha256 =
        format!("{:x}", Sha256::digest(&bytes[payload..bytes.len() - 32]));
    manifest.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&manifest.snapshot, bytes).unwrap();
}

#[test]
fn streamed_adapter_matches_comparison_plan() {
    let root = root("reference");
    let mut coarse = manifest(root.join("coarse.bin"), 4, "coarse");
    let mut fine = manifest(root.join("fine.bin"), 8, "fine");
    let coarse_fields = fields(coarse.domain().unwrap().layout(), 1.0);
    let fine_fields = fields(fine.domain().unwrap().layout(), 1.5);
    write(&mut coarse, &coarse_fields);
    write(&mut fine, &fine_fields);
    let left = decode::load(&coarse).unwrap();
    let right = decode::load(&fine).unwrap();
    let output = compare::compare(&coarse, &left, &fine, &right, 123).unwrap();
    let direct = ComparisonPlan::new(coarse.domain().unwrap(), fine.domain().unwrap())
        .unwrap()
        .compare(
            std::array::from_fn(|axis| coarse_fields[axis].as_slice()),
            std::array::from_fn(|axis| fine_fields[axis].as_slice()),
        )
        .unwrap();
    assert_eq!(output.full.l2, direct.full.l2);
    assert_eq!(output.full.h1, direct.full.h1);
    assert_eq!(output.full.vorticity_l2, direct.full.vorticity_l2);
    assert_eq!(output.full.divergence_l2, direct.full.divergence_l2);
    assert_eq!(output.common.l2, direct.common.l2);
    assert_eq!(output.newly_resolved.l2, direct.newly_resolved.l2);
    assert_eq!(output.mean_error, direct.mean_error);
    assert_ne!(left.coefficient_sha256, left.file_sha256);
    let zeros: [Vec<Complex64>; 3] = std::array::from_fn(|_| {
        vec![Complex64::new(0.0, 0.0); fine.domain().unwrap().layout().half_len()]
    });
    let absolute = ComparisonPlan::new(fine.domain().unwrap(), fine.domain().unwrap())
        .unwrap()
        .compare(
            std::array::from_fn(|axis| zeros[axis].as_slice()),
            std::array::from_fn(|axis| fine_fields[axis].as_slice()),
        )
        .unwrap();
    assert_eq!(output.fine_absolute.l2, absolute.full.l2);
    assert_eq!(output.fine_absolute.h1, absolute.full.h1);
    assert_eq!(
        output.fine_absolute.vorticity_l2,
        absolute.full.vorticity_l2
    );
    assert_eq!(
        output.fine_absolute.divergence_l2,
        absolute.full.divergence_l2
    );
}

#[test]
fn rejects_truncated_and_wrong_magic_without_state_allocation() {
    let root = root("framing");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    let bytes = fs::read(&item.snapshot).unwrap();
    fs::write(&item.snapshot, &bytes[..bytes.len() - 1]).unwrap();
    assert!(decode::load(&item).unwrap_err().contains("length mismatch"));
    let mut wrong = bytes;
    wrong[0] ^= 1;
    fs::write(&item.snapshot, wrong).unwrap();
    assert!(decode::load(&item).unwrap_err().contains("magic mismatch"));
}

#[test]
fn rejects_identity_hash_nonfinite_and_nyquist() {
    let root = root("payload");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let mut values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    item.identity = "different".into();
    assert!(decode::load(&item).unwrap_err().contains("length mismatch"));
    item.identity = "fixture".into();
    let mut damaged = fs::read(&item.snapshot).unwrap();
    let payload = 12 + 8 + item.identity.len() + 4 * 16;
    damaged[payload] ^= 1;
    fs::write(&item.snapshot, damaged).unwrap();
    assert!(decode::load(&item)
        .unwrap_err()
        .contains("trailer mismatch"));

    write(&mut item, &values);
    item.identity = "FIxture".into();
    assert!(decode::load(&item)
        .unwrap_err()
        .contains("identity mismatch"));
    item.identity = "fixture".into();

    values[0][0] = Complex64::new(f64::NAN, 0.0);
    write(&mut item, &values);
    assert!(decode::load(&item).unwrap_err().contains("InvalidSpectrum"));
    values[0][0] = Complex64::new(1.0, 0.0);
    let nyquist = item.domain().unwrap().layout().index([2, 0, 0]).unwrap();
    values[0][nyquist] = Complex64::new(1.0, 0.0);
    write(&mut item, &values);
    assert!(decode::load(&item).unwrap_err().contains("InvalidSpectrum"));
}

#[test]
fn manifest_and_command_enforce_schema_and_cap() {
    let root = root("command");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    left.snapshot = PathBuf::from("left.bin");
    right.snapshot = PathBuf::from("right.bin");
    left.plan = PathBuf::from("plan-left.json");
    right.plan = PathBuf::from("plan-right.json");
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.clone().into_os_string(),
        right_path.into_os_string(),
        cap.to_string().into(),
    ];
    let output = super::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-comparison-output-v1"));
    let mut refused = args.clone();
    refused[2] = "1".into();
    assert!(super::run(&refused).unwrap_err().contains("exceeds cap"));
    assert!(super::run(&[]).unwrap_err().contains("usage"));

    left.schema = "wrong".into();
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("manifest binding"));
    left.schema = "p10-snapshot-comparison-input-v1".into();
    left.evolution.schedule[0].step_ticks = 0;
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("piecewise schedule"));
    left.evolution.schedule[0].step_ticks = 32;
    left.plan_sha256 = "0".repeat(64);
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&left_path)
        .unwrap_err()
        .contains("plan SHA-256 mismatch"));
}

#[test]
fn rejects_profile_clock_domain_direction_and_cap() {
    let root = root("binding");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    right.evolution.quantum_exponent = -19;
    assert!(compare::compare(&left, &ls, &right, &rs, 0)
        .unwrap_err()
        .contains("semantics mismatch"));
    right.evolution = left.evolution.clone();
    let mut bad_clock = ClockHeader::from_manifest(&right);
    bad_clock.elapsed += 1;
    let bad = crate::model::Snapshot {
        clock: bad_clock,
        ..rs
    };
    assert!(compare::compare(&left, &ls, &right, &bad, 0)
        .unwrap_err()
        .contains("clock mismatch"));
    assert!(ComparisonPlan::new(right.domain().unwrap(), left.domain().unwrap()).is_err());
    assert!(decode::admitted_bytes(&left, &right).unwrap() > 1);
}
