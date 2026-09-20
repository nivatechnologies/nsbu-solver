//! Command, streaming-decode, framing and binding refusal coverage.

use super::*;

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
    let output = crate::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-comparison-output-v1"));
    let mut refused = args.clone();
    refused[2] = "1".into();
    assert!(crate::run(&refused).unwrap_err().contains("exceeds cap"));
    assert!(crate::run(&[]).unwrap_err().contains("usage"));

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
fn ordered_hessian_option_serializes_bound_output_and_rejects_clock_or_profile_gaps() {
    let root = root("hessian-command");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    for item in [&mut left, &mut right] {
        item.epoch = 2;
        item.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: item.identity.clone(),
        });
        item.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 2,
        });
    }
    let left_fields = fields(left.domain().unwrap().layout(), 1.0);
    let right_fields = fields(right.domain().unwrap().layout(), 1.5);
    write(&mut left, &left_fields);
    write(&mut right, &right_fields);
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.clone().into_os_string(),
        right_path.clone().into_os_string(),
        cap.to_string().into(),
        "--ordered-hessian".into(),
    ];
    let output = crate::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-ordered-hessian-diagnostic-output-v1"));
    assert!(output.contains("\"status\": \"not_assessed\""));

    right.epoch += 1;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(crate::run(&args)
        .unwrap_err()
        .contains("matched spatial clocks"));
    right.epoch = left.epoch;
    right.profile = None;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(crate::run(&args)
        .unwrap_err()
        .contains("exact profile binding"));

    right.profile = left.profile.clone();
    right.evolution.viscosity = 0.02;
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(crate::run(&args)
        .unwrap_err()
        .contains("evolution semantics mismatch"));
    right.evolution.viscosity = left.evolution.viscosity;
    right.evolution.schedule = vec![
        ScheduleSegment {
            from_inclusive: 0,
            until_exclusive: 32,
            step_ticks: 16,
        },
        ScheduleSegment {
            from_inclusive: 32,
            until_exclusive: 64,
            step_ticks: 32,
        },
    ];
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    assert!(crate::run(&args)
        .unwrap_err()
        .contains("evolution semantics mismatch"));
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

#[test]
fn matched_spatial_reports_different_guards_without_gating() {
    let root = root("spatial-guards");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 8, "right");
    left.admission_guard = Some(AdmissionGuard {
        advective_limit: 0.45,
        maximum_attempts: 128,
    });
    right.admission_guard = Some(AdmissionGuard {
        advective_limit: 3.3,
        maximum_attempts: 48,
    });
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.0);
    write(&mut left, &lf);
    write(&mut right, &rf);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    let output = compare::compare(&left, &ls, &right, &rs, 0).unwrap();
    assert_eq!(output.left_admission_guard.unwrap().advective_limit, 0.45);
    assert_eq!(output.right_admission_guard.unwrap().advective_limit, 3.3);
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("left_admission_guard"));
    assert!(json.contains("right_admission_guard"));

    right.admission_guard = None;
    let output = compare::compare(&left, &ls, &right, &rs, 0).unwrap();
    assert!(output.right_admission_guard.is_none());
}
