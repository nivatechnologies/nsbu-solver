//! Time-diagnostic admission, arithmetic-control and lineage refusals.

use super::*;

#[test]
fn time_diagnostic_accepts_h32_vs_piecewise_and_reports_scope() {
    let root = root("time-positive");
    let mut left = manifest(root.join("left.bin"), 4, "left-h32");
    let mut right = manifest(root.join("right.bin"), 4, "right-piecewise");
    right.source_commit = "b".repeat(40);
    right.backend = "fixture-w3-backend".into();
    right.execution = "fixture-w3-execution".into();
    enable_time(&mut left, &mut right, &root);
    let lf = fields(left.domain().unwrap().layout(), 1.0);
    let rf = fields(right.domain().unwrap().layout(), 1.25);
    write(&mut left, &lf);
    write(&mut right, &rf);
    left.snapshot = PathBuf::from("left.bin");
    right.snapshot = PathBuf::from("right.bin");
    left.plan = PathBuf::from("plan-left-h32.json");
    right.plan = PathBuf::from("plan-right-piecewise.json");
    let left_path = root.join("left.json");
    let right_path = root.join("right.json");
    fs::write(&left_path, serde_json::to_vec(&left).unwrap()).unwrap();
    fs::write(&right_path, serde_json::to_vec(&right).unwrap()).unwrap();
    let cap = decode::admitted_bytes(&left, &right).unwrap();
    let args = [
        left_path.into_os_string(),
        right_path.into_os_string(),
        cap.to_string().into(),
    ];
    let output = crate::run(&args).unwrap();
    assert!(output.contains("p10-snapshot-time-diagnostic-output-v1"));
    assert!(output.contains("\"comparison_kind\": \"TIME_DIAGNOSTIC\""));
    assert!(output.contains("\"status\": \"not_assessed\""));
    assert!(output.contains("\"accepted_windows\": 0"));
    assert!(output.contains("\"left_epoch\": 2"));
    assert!(output.contains("\"right_epoch\": 3"));
    assert!(output.contains("n4-m384-h32-cadv045-serial"));
    assert!(output.contains("n4-m384-piecewise-cadv33-w3"));
    assert!(output.contains("measured-n4-m384-serial-81bd"));
    assert!(output.contains("measured-n4-m384-w3-f13"));
    assert!(output.contains("reviewed-equivalence-supported-by-controls"));
    let mut under_cap = args.clone();
    under_cap[2] = "1".into();
    assert!(crate::run(&under_cap).unwrap_err().contains("exceeds cap"));
    let mut corrupted = fs::read(root.join("left.bin")).unwrap();
    corrupted[100] ^= 1;
    fs::write(root.join("left.bin"), corrupted).unwrap();
    assert!(crate::run(&args).is_err());
}

#[test]
fn time_diagnostic_rejects_every_immutable_semantic_change() {
    let root = root("time-immutable");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();
    let rejects =
        |candidate: &Manifest| compare::time_diagnostic(&left, &ls, candidate, &rs, 0).is_err();

    let mut changed = right.clone();
    changed.evolution.case_sha256 = "d".repeat(64);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.quantum_exponent += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.clock_target += 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.comparison_endpoint -= 1;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.lengths[0] = f64::from_bits(1.0_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.viscosity = f64::from_bits(0.01_f64.to_bits() + 1);
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.method = "other".into();
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.integration_force_dimensions = [383; 3];
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.absolute_tolerances[0] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.evolution.relative_tolerances[1] = 2e-5;
    assert!(rejects(&changed));
    changed = right.clone();
    changed.dimensions = [6; 3];
    assert!(rejects(&changed));
}

#[test]
fn time_diagnostic_rejects_mode_headers_guards_and_provenance() {
    let root = root("time-policy");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    let ls = decode::load(&left).unwrap();
    let rs = decode::load(&right).unwrap();

    let mut changed = right.clone();
    changed.comparison_kind = ComparisonKind::MatchedSpatial;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("kind mismatch"));
    changed = right.clone();
    changed.accepted_steps -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed.epoch -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed.evolution.schedule[1].from_inclusive -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("invalid time-diagnostic schedule"));
    changed = right.clone();
    changed.admission_guard.as_mut().unwrap().maximum_attempts -= 1;
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("derivation mismatch"));
    changed = right.clone();
    changed
        .arithmetic_control
        .as_mut()
        .unwrap()
        .review
        .reviewed_lineage
        .right
        .source_commit = "e".repeat(40);
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("binding mismatch"));
    changed = right.clone();
    changed.backend.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("side binding mismatch"));
    changed = right.clone();
    changed.execution.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("side binding mismatch"));
    changed = right.clone();
    changed.profile.as_mut().unwrap().value.push_str("-changed");
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("profile does not match"));
    changed = right.clone();
    changed.arithmetic_control.as_mut().unwrap().evidence_sha256 = "f".repeat(64);
    assert!(compare::time_diagnostic(&left, &ls, &changed, &rs, 0)
        .unwrap_err()
        .contains("binding mismatch"));

    left.snapshot = PathBuf::from("left.bin");
    left.plan = PathBuf::from("plan-left.json");
    let manifest_path = root.join("left.json");
    left.admission_guard.as_mut().unwrap().advective_limit = 0.0;
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("admission guard"));
    left.admission_guard.as_mut().unwrap().advective_limit = 0.45;
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 = "0".repeat(64);
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("arithmetic-control SHA-256 mismatch"));

    let evidence_path = root.join("serial-w3-arithmetic-review.json");
    let arbitrary = b"{\"arbitrary\":true}\n";
    fs::write(&evidence_path, arbitrary).unwrap();
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 =
        format!("{:x}", Sha256::digest(arbitrary));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("evidence schema"));

    let control = left.arithmetic_control.as_mut().unwrap();
    control.review.case_sha256 = "d".repeat(64);
    let mut semantic = serde_json::to_vec_pretty(&control.review).unwrap();
    semantic.push(b'\n');
    fs::write(&evidence_path, &semantic).unwrap();
    control.evidence_sha256 = format!("{:x}", Sha256::digest(&semantic));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    let reviewed_left = decode::read_manifest(&manifest_path).unwrap();
    let mut semantic_right = right.clone();
    semantic_right.arithmetic_control = reviewed_left.arithmetic_control.clone();
    assert!(
        compare::validate_manifest_pair(&reviewed_left, &semantic_right)
            .unwrap_err()
            .contains("physical contract mismatch")
    );
}

#[test]
fn arithmetic_review_accepts_64_kib_and_rejects_one_byte_more() {
    const CAP: usize = 64 * 1024;
    let root = root("arithmetic-cap");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    left.snapshot = PathBuf::from("left.bin");
    left.plan = PathBuf::from("plan-left.json");
    let manifest_path = root.join("left.json");
    let evidence_path = root.join("serial-w3-arithmetic-review.json");

    let control = left.arithmetic_control.as_mut().unwrap();
    let mut exact = serde_json::to_vec(&control.review).unwrap();
    exact.resize(CAP, b' ');
    fs::write(&evidence_path, &exact).unwrap();
    control.evidence_sha256 = format!("{:x}", Sha256::digest(&exact));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path).is_ok());

    exact.push(b' ');
    fs::write(&evidence_path, &exact).unwrap();
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 =
        format!("{:x}", Sha256::digest(&exact));
    fs::write(&manifest_path, serde_json::to_vec(&left).unwrap()).unwrap();
    assert!(decode::read_manifest(&manifest_path)
        .unwrap_err()
        .contains("exceeds 64 KiB bound"));
}

#[test]
fn legacy_profile_binding_requires_the_exact_complete_identity() {
    let root = root("legacy-profile");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    enable_time(&mut left, &mut right, &root);
    left.identity = "source=legacy;case=fixture;n=4;m=384;step=32".into();
    left.profile = Some(ProfileBinding {
        kind: ProfileBindingKind::LegacyFullIdentity,
        value: left.identity.clone(),
    });
    let control = left.arithmetic_control.as_mut().unwrap();
    control.review.reviewed_lineage.left.profile = left.profile.clone().unwrap();
    right.arithmetic_control = Some(control.clone());
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    left.profile
        .as_mut()
        .unwrap()
        .value
        .push_str(";invented=true");
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("profile does not match"));
    left.profile.as_mut().unwrap().value = left.identity.clone();
    left.profile.as_mut().unwrap().kind = ProfileBindingKind::IdentityProfileField;
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("profile does not match"));
}
