//! Fixed-schedule and matched-M512 spatial diagnostic coverage.

use super::*;

#[test]
fn force_resolution_diagnostic_is_closed_and_directional() {
    let root = root("force-diagnostic");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    enable_fixed_diagnostic(
        &mut left,
        &mut right,
        ComparisonKind::ForceResolutionDiagnostic,
    );
    assert_manifest_decodes(&root, "force-left.json", &left);
    assert_manifest_decodes(&root, "force-right.json", &right);
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_ne!(left.admission_guard, right.admission_guard);
    assert_eq!(left.accepted_steps, 8);
    assert_eq!(left.admission_guard.as_ref().unwrap().maximum_attempts, 48);

    let mut exhausted = right.clone();
    exhausted.admission_guard.as_mut().unwrap().maximum_attempts = 7;
    assert!(compare::validate_manifest_pair(&left, &exhausted).is_err());

    let mut changed = right.clone();
    changed.evolution.integration_force_dimensions = [384; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    let mut reversed_left = left.clone();
    let mut reversed_right = right.clone();
    reversed_left.evolution.integration_force_dimensions = [512; 3];
    reversed_right.evolution.integration_force_dimensions = [384; 3];
    assert!(compare::validate_manifest_pair(&reversed_left, &reversed_right).is_err());
    changed = right.clone();
    changed.evolution.method = "hochbruck-ostermann".into();
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    changed = right.clone();
    changed.dimensions = [383; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    assert_other_physics_rejected(&left, &right);
}

#[test]
fn matched_m512_spatial_diagnostic_is_closed_and_exactly_admitted() {
    let root = root("matched-m512-spatial");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 6, "right");
    write(&mut left, &fields(Layout::new([4; 3]).unwrap(), 1.0));
    write(&mut right, &fields(Layout::new([6; 3]).unwrap(), 1.25));
    enable_matched_m512_spatial(&mut left, &mut right);

    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_eq!(
        decode::admitted_bytes(&left, &right).unwrap(),
        4_600_889_344
    );

    for changed in [
        {
            let mut value = right.clone();
            value.source_commit = "a".repeat(40);
            value
        },
        {
            let mut value = right.clone();
            value.plan_sha256 = "a".repeat(64);
            value
        },
        {
            let mut value = right.clone();
            value.profile.as_mut().unwrap().value.push_str("-changed");
            value
        },
        {
            let mut value = right.clone();
            value.identity.push_str(";source=duplicate");
            value
        },
        {
            let mut value = right.clone();
            value.identity.push_str(";production_source=duplicate");
            value
        },
        {
            let mut value = right.clone();
            value.dimensions = [384; 3];
            value
        },
        {
            let mut value = right.clone();
            value.evolution.schedule[1].step_ticks = 64;
            value
        },
        {
            let mut value = right.clone();
            value.evolution.relative_tolerances[0] = 2e-5;
            value
        },
    ] {
        assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    }

    let mut changed = right.clone();
    changed.comparison_kind = ComparisonKind::MatchedSpatial;
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
}

#[test]
fn matched_m512_kind_has_a_distinct_small_decoder_route() {
    let root = root("matched-m512-decoder");
    let mut input = manifest(root.join("state.bin"), 4, "fixture");
    let layout = input.domain().unwrap().layout();
    write(&mut input, &fields(layout, 1.0));
    input.comparison_kind = ComparisonKind::MatchedM512SpatialDiagnostic;
    input.evolution.integration_force_dimensions = [512; 3];
    assert_manifest_decodes(&root, "m512-kind.json", &input);

    input.comparison_kind = ComparisonKind::MatchedSpatial;
    let path = root.join("old-kind.json");
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(decode::read_manifest(&path).is_err());
}

#[test]
fn method_diagnostic_is_closed_and_directional() {
    let root = root("method-diagnostic");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 4, "right");
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    enable_fixed_diagnostic(&mut left, &mut right, ComparisonKind::MethodDiagnostic);
    assert_manifest_decodes(&root, "method-left.json", &left);
    assert_manifest_decodes(&root, "method-right.json", &right);
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_ne!(left.admission_guard, right.admission_guard);
    assert_eq!(right.accepted_steps, 8);
    assert_eq!(right.admission_guard.as_ref().unwrap().maximum_attempts, 48);

    let mut exhausted = right.clone();
    exhausted.admission_guard.as_mut().unwrap().maximum_attempts = 7;
    assert!(compare::validate_manifest_pair(&left, &exhausted).is_err());

    let mut changed = right.clone();
    changed.evolution.method = "cox-matthews".into();
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    let mut reversed_left = left.clone();
    let mut reversed_right = right.clone();
    reversed_left.evolution.method = "hochbruck-ostermann".into();
    reversed_right.evolution.method = "cox-matthews".into();
    assert!(compare::validate_manifest_pair(&reversed_left, &reversed_right).is_err());
    changed = right.clone();
    changed.evolution.integration_force_dimensions = [512; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    changed = right.clone();
    changed.dimensions = [256; 3];
    assert!(compare::validate_manifest_pair(&left, &changed).is_err());
    assert_other_physics_rejected(&left, &right);
}

#[test]
fn fixed_diagnostics_serialize_tiny_spectrum_outputs() {
    for (kind, schema, label) in [
        (
            ComparisonKind::ForceResolutionDiagnostic,
            "p10-snapshot-force-resolution-diagnostic-output-v1",
            "FORCE_RESOLUTION_DIAGNOSTIC",
        ),
        (
            ComparisonKind::MethodDiagnostic,
            "p10-snapshot-method-diagnostic-output-v1",
            "METHOD_DIAGNOSTIC",
        ),
    ] {
        let root = root(label);
        let mut left = manifest(root.join("left.bin"), 4, "left");
        let mut right = manifest(root.join("right.bin"), 4, "right");
        left.comparison_kind = kind;
        right.comparison_kind = kind;
        left.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: left.identity.clone(),
        });
        right.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::LegacyFullIdentity,
            value: right.identity.clone(),
        });
        left.admission_guard = Some(AdmissionGuard {
            advective_limit: 0.45,
            maximum_attempts: 48,
        });
        right.admission_guard = Some(AdmissionGuard {
            advective_limit: 3.3,
            maximum_attempts: 48,
        });
        for manifest in [&mut left, &mut right] {
            manifest.evolution.clock_target = 256;
            manifest.evolution.comparison_endpoint = 256;
            manifest.evolution.schedule[0].until_exclusive = 256;
            manifest.elapsed = 256;
            manifest.target = 256;
            manifest.epoch = 8;
            manifest.accepted_steps = 8;
        }
        match kind {
            ComparisonKind::ForceResolutionDiagnostic => {
                right.evolution.integration_force_dimensions = [512; 3];
            }
            ComparisonKind::MethodDiagnostic => {
                right.evolution.method = "hochbruck-ostermann".into();
            }
            _ => unreachable!(),
        }
        let left_values = fields(left.domain().unwrap().layout(), 1.0);
        let right_values = fields(right.domain().unwrap().layout(), 1.25);
        write(&mut left, &left_values);
        write(&mut right, &right_values);
        let left_state = decode::load(&left).unwrap();
        let right_state = decode::load(&right).unwrap();
        let output = compare::diagnostic_output(
            &left,
            &left_state,
            &right,
            &right_state,
            123,
            schema,
            label,
            None,
        )
        .unwrap();
        let json = serde_json::to_value(output).unwrap();
        assert_eq!(json["schema"], schema);
        assert_eq!(json["comparison_kind"], label);
        assert_eq!(json["acceptance"]["status"], "not_assessed");
        assert_eq!(json["acceptance"]["accepted_windows"], 0);
        assert!(json.get("arithmetic_control").is_none());
        assert_eq!(json["left_identity"], "left");
        assert_eq!(json["right_identity"], "right");
        assert_eq!(json["left_admission_guard"]["advective_limit"], 0.45);
        assert_eq!(json["right_admission_guard"]["advective_limit"], 3.3);
        assert_eq!(json["left_plan_sha256"], left.plan_sha256);
        assert_eq!(json["right_plan_sha256"], right.plan_sha256);
        assert_eq!(json["clock"]["left_accepted_steps"], 8);
        assert_eq!(json["clock"]["right_accepted_steps"], 8);
        assert_eq!(json["left_admission_guard"]["maximum_attempts"], 48);
        assert_eq!(json["right_admission_guard"]["maximum_attempts"], 48);
        assert_eq!(json["admitted_bytes"], 123);
    }
}
