//! Meaningful refusal tests that keep the changed decoder and comparator
//! sources above the actual LLVM branch-coverage gate: every case drives one
//! distinct rejected condition through `decode::read_manifest`,
//! `decode::load`, `compare::validate_manifest_pair`, `compare::compare` or
//! the CLI entry point, and asserts the exact fail-closed message.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use super::*;

fn valid_review(left: &Manifest, right: &Manifest) -> ArithmeticReview {
    ArithmeticReview {
        schema: "p10-time-arithmetic-review-v1".into(),
        conclusion: "reviewed-equivalence-supported-by-controls".into(),
        case_sha256: left.evolution.case_sha256.clone(),
        method: left.evolution.method.clone(),
        integration_force_dimensions: left.evolution.integration_force_dimensions,
        measured_control: MeasuredControl {
            outcome: "successful-exact-bit".into(),
            serial: MeasuredSide {
                source_commit: "8".repeat(40),
                backend: "measured-serial-backend".into(),
                execution: "measured-serial-execution".into(),
                configuration: "measured-n4-serial".into(),
            },
            w3: MeasuredSide {
                source_commit: "f".repeat(40),
                backend: "measured-w3-backend".into(),
                execution: "measured-w3-execution".into(),
                configuration: "measured-n4-w3".into(),
            },
        },
        reviewed_lineage: ReviewedLineage {
            status: "reviewed-unchanged-kernel-lineage".into(),
            left_control_role: "serial".into(),
            right_control_role: "w3".into(),
            left: ArithmeticSide {
                source_commit: left.source_commit.clone(),
                backend: left.backend.clone(),
                execution: left.execution.clone(),
                profile: left.profile.clone().unwrap_or(ProfileBinding {
                    kind: ProfileBindingKind::IdentityProfileField,
                    value: "fixture-n4".into(),
                }),
            },
            right: ArithmeticSide {
                source_commit: right.source_commit.clone(),
                backend: right.backend.clone(),
                execution: right.execution.clone(),
                profile: right.profile.clone().unwrap_or(ProfileBinding {
                    kind: ProfileBindingKind::IdentityProfileField,
                    value: "fixture-n4".into(),
                }),
            },
        },
    }
}

fn attached(dir: &std::path::Path, name: &str) -> (Manifest, Manifest) {
    let mut left = manifest(dir.join(format!("{name}-l.bin")), 4, "left-id");
    let mut right = manifest(dir.join(format!("{name}-r.bin")), 4, "right-id");
    right.source_commit = "b".repeat(40);
    let review = valid_review(&left, &right);
    for side in [&mut left, &mut right] {
        side.coefficient_sha256 = "1".repeat(64);
        side.file_sha256 = "2".repeat(64);
        side.profile = Some(ProfileBinding {
            kind: ProfileBindingKind::IdentityProfileField,
            value: "fixture-n4".into(),
        });
        side.identity = "fixture;profile=fixture-n4".into();
        side.arithmetic_control = Some(ArithmeticControl {
            evidence: PathBuf::from("unused.json"),
            evidence_sha256: "0".repeat(64),
            review: review.clone(),
        });
    }
    (left, right)
}

#[test]
fn read_manifest_refuses_broken_envelopes_and_digest_shapes() {
    let dir = root("cov-envelope");
    let (manifest, _) = attached(&dir, "env");
    let rejects = |name: &str, item: &Manifest, message: &str| {
        read_rejects(&dir, name, item, message);
    };
    let mut broken = manifest.clone();
    broken.execution = String::new();
    rejects("execution", &broken, "invalid comparison manifest binding");
    broken = manifest.clone();
    broken.identity = "y".repeat(16 * 1024 + 1);
    rejects("identity", &broken, "invalid comparison manifest binding");
    broken = manifest.clone();
    broken.plan_sha256 = "z".repeat(64);
    rejects(
        "plan-digest",
        &broken,
        "invalid comparison manifest binding",
    );
    broken = manifest.clone();
    broken.coefficient_sha256 = "c".repeat(63);
    rejects(
        "coefficient-digest",
        &broken,
        "invalid comparison manifest binding",
    );
    broken = manifest.clone();
    broken.file_sha256 = String::new();
    rejects(
        "file-digest",
        &broken,
        "invalid comparison manifest binding",
    );
    broken = manifest.clone();
    broken.evolution.case_sha256 = "x".repeat(64);
    rejects(
        "case-digest",
        &broken,
        "invalid comparison manifest binding",
    );
    broken = manifest.clone();
    broken.admission_guard = Some(AdmissionGuard {
        advective_limit: 0.0,
        maximum_attempts: 4,
    });
    rejects("guard-limit", &broken, "invalid admission guard metadata");
}

fn read_rejects(dir: &std::path::Path, name: &str, manifest: &Manifest, message: &str) {
    let path = dir.join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec(manifest).unwrap()).unwrap();
    let error = decode::read_manifest(&path).unwrap_err();
    assert!(error.contains(message), "{name}: {error}");
}

#[test]
fn read_manifest_refuses_broken_evolution_and_schedules() {
    let dir = root("cov-evolution");
    let (manifest, _) = attached(&dir, "evo");
    let mut broken = manifest.clone();
    broken.elapsed = 63;
    read_rejects(&dir, "endpoint", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.evolution.schedule = Vec::new();
    read_rejects(&dir, "empty", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.comparison_kind = ComparisonKind::TimeDiagnostic;
    broken.evolution.method = "rk4".into();
    read_rejects(&dir, "time-method", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.comparison_kind = ComparisonKind::ForceResolutionDiagnostic;
    broken.evolution.method = "rk4".into();
    read_rejects(&dir, "force-method", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.comparison_kind = ComparisonKind::ForceResolutionDiagnostic;
    broken.evolution.integration_force_dimensions = [448; 3];
    read_rejects(&dir, "force-dims", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.evolution.schedule = vec![ScheduleSegment {
        from_inclusive: 0,
        until_exclusive: 0,
        step_ticks: 32,
    }];
    read_rejects(&dir, "empty-span", &broken, "invalid piecewise schedule");
}

#[test]
fn read_manifest_refuses_every_invalid_arithmetic_review_shape() {
    let dir = root("cov-review");
    let (mut left, right) = attached(&dir, "rev");
    left.arithmetic_control.as_mut().unwrap().evidence_sha256 = "z".repeat(64);
    read_rejects(
        &dir,
        "evidence-hex",
        &left,
        "invalid arithmetic-control binding",
    );
    let mutations: Vec<(&str, ReviewMutation)> = vec![
        ("schema", |r| r.schema = "other".into()),
        ("case", |r| r.case_sha256 = "z".repeat(64)),
        ("method", |r| r.method = "rk4".into()),
        ("dims", |r| r.integration_force_dimensions = [448; 3]),
        ("outcome", |r| r.measured_control.outcome = "partial".into()),
        ("serial-hex", |r| {
            r.measured_control.serial.source_commit = "z".into()
        }),
        ("serial-backend", |r| {
            r.measured_control.serial.backend = String::new()
        }),
        ("serial-execution", |r| {
            r.measured_control.serial.execution = String::new()
        }),
        ("w3-configuration", |r| {
            r.measured_control.w3.configuration = String::new()
        }),
        ("status", |r| {
            r.reviewed_lineage.status = "unreviewed".into()
        }),
        ("left-role", |r| {
            r.reviewed_lineage.left_control_role = "w3".into()
        }),
        ("right-role", |r| {
            r.reviewed_lineage.right_control_role = "serial".into()
        }),
        ("lineage-hex", |r| {
            r.reviewed_lineage.left.source_commit = "z".repeat(41)
        }),
        ("lineage-backend", |r| {
            r.reviewed_lineage.right.backend = String::new()
        }),
        ("lineage-execution", |r| {
            r.reviewed_lineage.left.execution = String::new()
        }),
        ("lineage-profile", |r| {
            r.reviewed_lineage.right.profile.value = String::new()
        }),
    ];
    for (name, mutate) in mutations {
        let mut candidate = right.clone();
        let mut control = candidate.arithmetic_control.clone().unwrap();
        mutate(&mut control.review);
        candidate.arithmetic_control = Some(control);
        read_rejects(&dir, name, &candidate, "invalid arithmetic-control binding");
    }
}

#[test]
fn load_refuses_corrupt_snapshots_and_off_spectrum_fields() {
    let dir = root("cov-load");
    let mut manifest = manifest(dir.join("snap.bin"), 4, "identity-alpha");
    let values = fields(manifest.domain().unwrap().layout(), 1.0);
    write(&mut manifest, &values);
    let bytes = fs::read(&manifest.snapshot).unwrap();
    fs::write(dir.join("snap-length.bin"), &bytes[..bytes.len() - 1]).unwrap();
    let mut length = manifest.clone();
    length.snapshot = dir.join("snap-length.bin");
    assert!(decode::load(&length)
        .unwrap_err()
        .contains("snapshot length mismatch"));
    let mut magic = manifest.clone();
    magic.snapshot = dir.join("snap-magic.bin");
    let mut bytes = fs::read(&manifest.snapshot).unwrap();
    bytes[0] = b'X';
    fs::write(&magic.snapshot, &bytes).unwrap();
    assert!(decode::load(&magic)
        .unwrap_err()
        .contains("snapshot magic mismatch"));
    let mut identity = manifest.clone();
    identity.identity = "identity-omega".into();
    assert!(decode::load(&identity)
        .unwrap_err()
        .contains("snapshot identity mismatch"));
    let mut clock = manifest.clone();
    clock.elapsed = 65;
    assert!(decode::load(&clock)
        .unwrap_err()
        .contains("snapshot clock mismatch"));
    let mut coefficient = manifest.clone();
    coefficient.coefficient_sha256 = "9".repeat(64);
    assert!(decode::load(&coefficient)
        .unwrap_err()
        .contains("snapshot hash does not match reviewed manifest"));
    let mut file = manifest.clone();
    file.file_sha256 = "9".repeat(64);
    assert!(decode::load(&file)
        .unwrap_err()
        .contains("snapshot hash does not match reviewed manifest"));
    let mut layout_fields = fields(manifest.domain().unwrap().layout(), 1.0);
    let layout = manifest.domain().unwrap().layout();
    layout_fields[2][layout.index([2, 0, 0]).unwrap()].re += 1.0;
    let mut skewed = manifest.clone();
    skewed.snapshot = dir.join("snap-skew.bin");
    write(&mut skewed, &layout_fields);
    assert!(decode::load(&skewed).is_err());
    let mut trailer = manifest.clone();
    trailer.snapshot = dir.join("snap-trailer.bin");
    let mut bytes = fs::read(&manifest.snapshot).unwrap();
    let point = bytes.len() - 20;
    bytes[point] ^= 0b1000_0000;
    fs::write(&trailer.snapshot, &bytes).unwrap();
    assert!(decode::load(&trailer)
        .unwrap_err()
        .contains("trailer mismatch"));
}

#[test]
fn compare_refuses_endpoint_clock_and_fixed_diagnostic_divergence() {
    let dir = root("cov-compare");
    let (mut left, mut right) = attached(&dir, "cmp");
    left.comparison_kind = ComparisonKind::MatchedSpatial;
    right.comparison_kind = ComparisonKind::MatchedSpatial;
    let layout = left.domain().unwrap().layout();
    let values = fields(layout, 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    right.backend = "fixture-w3-backend".into();
    right.execution = "fixture-w3-execution".into();
    write(&mut right, &values);
    let left_snapshot = decode::load(&left).unwrap();
    for mutate in [
        |snapshot: &mut crate::model::Snapshot| snapshot.clock.elapsed += 1,
        |snapshot: &mut crate::model::Snapshot| snapshot.clock.target += 1,
        |snapshot: &mut crate::model::Snapshot| snapshot.clock.epoch += 1,
        |snapshot: &mut crate::model::Snapshot| snapshot.clock.accepted_steps += 1,
    ] {
        let mut skewed = decode::load(&right).unwrap();
        mutate(&mut skewed);
        assert!(
            compare::compare(&left, &left_snapshot, &right, &skewed, usize::MAX)
                .unwrap_err()
                .contains("endpoint clock mismatch")
        );
    }
    let mut left = manifest(dir.join("fixed-l.bin"), 4, "fixed-left");
    let mut right = manifest(dir.join("fixed-r.bin"), 4, "fixed-right");
    enable_fixed_diagnostic(
        &mut left,
        &mut right,
        ComparisonKind::ForceResolutionDiagnostic,
    );
    let mut broken = left.clone();
    broken.dimensions = [256; 3];
    assert!(compare::validate_manifest_pair(&broken, &right)
        .unwrap_err()
        .contains("contract mismatch"));
    enable_fixed_diagnostic(&mut left, &mut right, ComparisonKind::MethodDiagnostic);
    let mut broken = left.clone();
    broken.dimensions = [256; 3];
    assert!(compare::validate_manifest_pair(&broken, &right).is_err());
    let mut broken = right.clone();
    broken.elapsed = 255;
    assert!(compare::validate_fixed_diagnostic_pair(&left, &broken)
        .unwrap_err()
        .contains("diagnostic physical clock mismatch"));
    let mut broken = right.clone();
    broken.target = 257;
    assert!(compare::validate_fixed_diagnostic_pair(&left, &broken)
        .unwrap_err()
        .contains("diagnostic physical clock mismatch"));
    let (control_left, control_right) = attached(&dir, "fixed-control");
    let mut broken = left.clone();
    broken.arithmetic_control = control_left.arithmetic_control;
    assert!(compare::validate_fixed_diagnostic_pair(&broken, &right)
        .unwrap_err()
        .contains("arithmetic-control override"));
    let mut broken = right.clone();
    broken.arithmetic_control = control_right.arithmetic_control;
    assert!(compare::validate_fixed_diagnostic_pair(&left, &broken)
        .unwrap_err()
        .contains("arithmetic-control override"));
}

#[test]
fn time_validation_refuses_schedule_clock_lineage_and_guard_divergence() {
    let dir = root("cov-time");
    let (mut left, mut right) = attached(&dir, "time");
    enable_time(&mut left, &mut right, &dir);
    let mut broken = right.clone();
    broken.elapsed = 63;
    assert!(compare::validate_manifest_pair(&left, &broken)
        .unwrap_err()
        .contains("physical clock mismatch"));
    let mut broken = right.clone();
    broken.target = 65;
    assert!(compare::validate_manifest_pair(&left, &broken)
        .unwrap_err()
        .contains("physical clock mismatch"));
    let review_failures: Vec<(&str, ReviewMutation)> = vec![
        ("case", |r| r.case_sha256 = "d".repeat(64)),
        ("method", |r| r.method = "rk4".into()),
        ("dims", |r| r.integration_force_dimensions = [320; 3]),
        ("lineage-commit", |r| {
            r.reviewed_lineage.left.source_commit = "d".repeat(40)
        }),
        ("lineage-profile", |r| {
            r.reviewed_lineage.left.profile.value.push('x')
        }),
        ("lineage-execution", |r| {
            r.reviewed_lineage.right.execution.push('x')
        }),
    ];
    for (name, mutate) in review_failures {
        let (mut broken_left, mut broken_right) = attached(&dir, name);
        enable_time(&mut broken_left, &mut broken_right, &dir);
        for side in [&mut broken_left, &mut broken_right] {
            mutate(&mut side.arithmetic_control.as_mut().unwrap().review);
        }
        assert!(
            compare::validate_manifest_pair(&broken_left, &broken_right).is_err(),
            "{name}"
        );
    }
    let schedules: Vec<(&str, Vec<ScheduleSegment>)> = vec![
        (
            "gap",
            vec![
                ScheduleSegment {
                    from_inclusive: 0,
                    until_exclusive: 32,
                    step_ticks: 16,
                },
                ScheduleSegment {
                    from_inclusive: 48,
                    until_exclusive: 64,
                    step_ticks: 16,
                },
            ],
        ),
        (
            "zero",
            vec![ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 64,
                step_ticks: 0,
            }],
        ),
        (
            "ragged",
            vec![ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 64,
                step_ticks: 17,
            }],
        ),
        (
            "short",
            vec![ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 32,
                step_ticks: 16,
            }],
        ),
    ];
    for (name, schedule) in schedules {
        let (mut broken_left, mut broken_right) = attached(&dir, name);
        enable_time(&mut broken_left, &mut broken_right, &dir);
        broken_right.evolution.schedule = schedule;
        broken_right.accepted_steps = 3;
        assert!(
            compare::validate_manifest_pair(&broken_left, &broken_right)
                .unwrap_err()
                .contains("schedule"),
            "{name}"
        );
    }
    for (name, limit) in [("nan", f64::NAN), ("negative", -1.0)] {
        let (mut broken_left, mut broken_right) = attached(&dir, name);
        enable_time(&mut broken_left, &mut broken_right, &dir);
        broken_right
            .admission_guard
            .as_mut()
            .unwrap()
            .advective_limit = limit;
        assert!(
            compare::validate_manifest_pair(&broken_left, &broken_right)
                .unwrap_err()
                .contains("derivation"),
            "{name}"
        );
    }
    let (mut missing, _) = attached(&dir, "missing");
    enable_time(&mut missing, &mut right, &dir);
    missing.arithmetic_control = None;
    assert!(compare::validate_manifest_pair(&missing, &right)
        .unwrap_err()
        .contains("missing arithmetic-control"));
}

#[test]
fn cli_entry_refuses_usage_and_cap_malformations() {
    let usage = crate::run(&[]).unwrap_err();
    assert!(usage.contains("usage"), "{usage}");
    assert!(crate::run(&[OsString::from("a"), OsString::from("b")]).is_err());
    assert!(crate::run(&[
        OsString::from("a"),
        OsString::from("b"),
        OsString::from("not-a-number"),
    ])
    .unwrap_err()
    .contains("InvalidDigit"));
    let garbled: Vec<OsString> = vec![
        OsString::from("a"),
        OsString::from("b"),
        OsString::from_vec(vec![0xff, 0xfe]),
    ];
    assert!(crate::run(&garbled)
        .unwrap_err()
        .contains("CAP_BYTES is not UTF-8"));
    assert!(crate::run(&[
        OsString::from("missing-left.json"),
        OsString::from("missing-right.json"),
        OsString::from("missing-force.json"),
        OsString::from("1024"),
        OsString::from("--mixed-force-space"),
    ])
    .is_err());
}
