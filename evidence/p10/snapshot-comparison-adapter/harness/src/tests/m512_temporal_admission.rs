//! Decoder admission of every synthetic M512 temporal fixture side through
//! `decode::read_manifest`: each byte-exact producer fixture carries synthetic
//! identities and epoch records and is staged beside co-located arithmetic
//! evidence and a valid synthetic capture plan that is hash-bound by resealing
//! the fixture's `plan_sha256` to the plan bytes' actual SHA-256. These
//! fixtures demonstrate decoder admission only — never a real capture,
//! comparison or window result. The h64--h32 sides are additionally retargeted
//! through post-2048 nested schedules, and the refusal branches below keep the
//! changed decoder code adversarially covered.

use super::*;

const H64_CLOCK0512: &[u8] = include_bytes!("m512-temporal/h64h32-clock0512-h64.json");
const H32_CLOCK0512: &[u8] = include_bytes!("m512-temporal/h64h32-clock0512-h32.json");
const H32_CLOCK2560: &[u8] = include_bytes!("m512-temporal/h32h16-clock2560-h32.json");
const H16_CLOCK2560: &[u8] = include_bytes!("m512-temporal/h32h16-clock2560-h16.json");

fn synthetic_plan(branch: &str) -> Vec<u8> {
    format!("{{\"synthetic-reviewed-capture-plan\":\"{branch}\"}}\n").into_bytes()
}

fn stage_side(base: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = base.join("a/b/c/d");
    fs::create_dir_all(&dir).unwrap();
    let mut manifest: Manifest = serde_json::from_slice(bytes).unwrap();
    let control = manifest.arithmetic_control.clone().unwrap();
    let mut evidence = serde_json::to_vec_pretty(&control.review).unwrap();
    evidence.push(b'\n');
    assert_eq!(
        format!("{:x}", Sha256::digest(&evidence)),
        control.evidence_sha256,
        "fixture evidence must already be hash-sealed for {name}"
    );
    fs::write(dir.join(&control.evidence), &evidence).unwrap();
    let plan = synthetic_plan(name);
    manifest.plan_sha256 = format!("{:x}", Sha256::digest(&plan));
    let relative = manifest.plan.strip_prefix("../../../../").unwrap();
    let staged = base.join(relative);
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &plan).unwrap();
    let path = dir.join("input.json");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    path
}

fn retarget_post_2048(manifest: &mut Manifest, endpoint: u128, steps: [u128; 2]) {
    manifest.evolution.schedule = vec![
        ScheduleSegment {
            from_inclusive: 0,
            until_exclusive: 2048,
            step_ticks: steps[0],
        },
        ScheduleSegment {
            from_inclusive: 2048,
            until_exclusive: endpoint,
            step_ticks: steps[1],
        },
    ];
    manifest.evolution.comparison_endpoint = endpoint;
    manifest.elapsed = endpoint;
}

#[test]
fn hash_bound_synthetic_plans_admit_all_four_m512_sides() {
    for (name, bytes) in [
        ("h64", H64_CLOCK0512),
        ("h32-early", H32_CLOCK0512),
        ("h32", H32_CLOCK2560),
        ("h16", H16_CLOCK2560),
    ] {
        let base = root(&format!("admission-{name}"));
        let path = stage_side(&base, name, bytes);
        let manifest = decode::read_manifest(&path).unwrap_or_else(|error| {
            panic!("{name} must admit with a hash-bound synthetic plan: {error}")
        });
        assert_eq!(manifest.comparison_kind, ComparisonKind::TimeDiagnostic);
        assert_eq!(manifest.dimensions, [512; 3]);
        assert_eq!(manifest.evolution.integration_force_dimensions, [512; 3]);
        assert!(manifest.arithmetic_control.is_some());
    }
}

#[test]
fn both_pairs_admit_through_post_2048_nested_schedules() {
    let base = root("admission-post-2048");
    let sides = [
        ("h64", H64_CLOCK0512, [64_u128, 128]),
        ("h32", H32_CLOCK0512, [32_u128, 64]),
        ("h16", H16_CLOCK2560, [16_u128, 32]),
    ];
    for (name, bytes, steps) in sides {
        let path = stage_side(&base, name, bytes);
        let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        retarget_post_2048(&mut manifest, 2560, steps);
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let read = decode::read_manifest(&path)
            .unwrap_or_else(|error| panic!("{name} post-2048 schedule: {error}"));
        assert_eq!(read.evolution.schedule[0].until_exclusive, 2048);
        assert_eq!(read.evolution.schedule[1].from_inclusive, 2048);
        assert_eq!(read.evolution.comparison_endpoint, read.elapsed);
    }
    let unmodified = stage_side(&root("admission-native-2560"), "h32", H32_CLOCK2560);
    let read = decode::read_manifest(&unmodified).unwrap();
    assert_eq!(read.evolution.comparison_endpoint, 2560);
    assert_eq!(read.evolution.schedule[1].from_inclusive, 2048);
}

#[test]
fn read_manifest_refuses_tampered_or_divergent_arithmetic_evidence() {
    let base = root("admission-evidence-refusals");
    let path = stage_side(&base, "h64", H64_CLOCK0512);
    let evidence = path.parent().unwrap().join("arithmetic-evidence.json");
    fs::write(&evidence, b"{\"tampered\":true}\n").unwrap();
    assert!(decode::read_manifest(&path)
        .unwrap_err()
        .contains("arithmetic-control SHA-256 mismatch"));
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut valid =
        serde_json::to_vec_pretty(&manifest.arithmetic_control.as_ref().unwrap().review).unwrap();
    valid.push(b'\n');
    fs::write(&evidence, &valid).unwrap();
    manifest
        .arithmetic_control
        .as_mut()
        .unwrap()
        .review
        .conclusion = "unreviewed-equivalence-claim".into();
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(decode::read_manifest(&path)
        .unwrap_err()
        .contains("invalid arithmetic-control binding"));
    let diverged = base.join("a/b/c/d/other.json");
    let mut manifest: Manifest = serde_json::from_slice(H64_CLOCK0512).unwrap();
    manifest.plan_sha256 = format!("{:x}", Sha256::digest(synthetic_plan("h64")));
    let mut review_value: serde_json::Value =
        serde_json::to_value(manifest.arithmetic_control.as_ref().unwrap().review.clone()).unwrap();
    review_value["case_sha256"] = serde_json::json!("d".repeat(64));
    let mut foreign = serde_json::to_vec_pretty(&review_value).unwrap();
    foreign.push(b'\n');
    manifest
        .arithmetic_control
        .as_mut()
        .unwrap()
        .evidence_sha256 = format!("{:x}", Sha256::digest(&foreign));
    fs::write(&diverged, serde_json::to_vec(&manifest).unwrap()).unwrap();
    fs::write(&evidence, &foreign).unwrap();
    assert!(decode::read_manifest(&diverged)
        .unwrap_err()
        .contains("arithmetic-control evidence content mismatch"));
    manifest
        .arithmetic_control
        .as_mut()
        .unwrap()
        .evidence_sha256 = format!("{:x}", Sha256::digest(b"[1, not json"));
    fs::write(&diverged, serde_json::to_vec(&manifest).unwrap()).unwrap();
    fs::write(&evidence, b"[1, not json").unwrap();
    assert!(decode::read_manifest(&diverged)
        .unwrap_err()
        .contains("invalid arithmetic-control evidence schema"));
}

#[test]
fn read_manifest_refuses_malformed_post_2048_schedules_and_bindings() {
    let base = root("admission-schedule-refusals");
    let path = stage_side(&base, "h64", H64_CLOCK0512);
    let refusals = [
        (
            "gap",
            vec![(0_u128, 2048, 64), (2112, 2560, 64)],
            "invalid piecewise schedule",
        ),
        (
            "zero-step",
            vec![(0, 2048, 0), (2048, 2560, 64)],
            "invalid piecewise schedule",
        ),
        (
            "ragged",
            vec![(0, 2048, 64), (2048, 2560, 66)],
            "invalid piecewise schedule",
        ),
        (
            "short",
            vec![(0, 2048, 64), (2048, 2496, 64)],
            "does not reach target",
        ),
    ];
    for (name, spans, message) in refusals {
        let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest.evolution.comparison_endpoint = 2560;
        manifest.elapsed = 2560;
        manifest.evolution.schedule = spans
            .into_iter()
            .map(
                |(from_inclusive, until_exclusive, step_ticks)| ScheduleSegment {
                    from_inclusive,
                    until_exclusive,
                    step_ticks,
                },
            )
            .collect();
        let broken = base.join(format!("{name}.json"));
        fs::write(&broken, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(decode::read_manifest(&broken)
            .unwrap_err()
            .contains(message));
    }
}

#[test]
fn read_manifest_refuses_envelope_tolerance_and_bound_violations() {
    let base = root("admission-envelope-refusals");
    let staged = stage_side(&base, "h64", H64_CLOCK0512);
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&staged).unwrap()).unwrap();
    let rejects = |name: &str, manifest: &Manifest, message: &str| {
        let path = base.join(format!("{name}.json"));
        fs::write(&path, serde_json::to_vec(manifest).unwrap()).unwrap();
        let error = decode::read_manifest(&path).unwrap_err();
        assert!(error.contains(message), "{name}: {error}");
    };
    let mut broken = manifest.clone();
    broken.schema = "p10-other-v1".into();
    rejects("schema", &broken, "invalid comparison manifest binding");
    broken = manifest.clone();
    broken.backend = String::new();
    rejects("backend", &broken, "invalid comparison manifest binding");
    broken = manifest.clone();
    broken.source_commit = "e25f".into();
    rejects("commit", &broken, "invalid comparison manifest binding");
    broken = manifest.clone();
    broken.evolution.clock_target += 1;
    rejects("clock", &broken, "invalid evolution semantics");
    broken = manifest.clone();
    broken.evolution.relative_tolerances[0] = -1.0;
    rejects("tolerance", &broken, "invalid evolution tolerances");
    broken = manifest.clone();
    broken.admission_guard = Some(AdmissionGuard {
        advective_limit: -1.0,
        maximum_attempts: 48,
    });
    rejects("guard-nan", &broken, "invalid admission guard metadata");
    broken = manifest.clone();
    broken.admission_guard = Some(AdmissionGuard {
        advective_limit: 3.3,
        maximum_attempts: 0,
    });
    rejects("guard-zero", &broken, "invalid admission guard metadata");
    let mut oversized = manifest.clone();
    oversized.identity = "x".repeat(64 * 1024 + 8);
    let path = base.join("oversized.json");
    fs::write(&path, serde_json::to_vec(&oversized).unwrap()).unwrap();
    assert!(decode::read_manifest(&path)
        .unwrap_err()
        .contains("manifest exceeds 64 KiB bound"));
    let bloated = base.join("bloated-plan.json");
    fs::write(&bloated, vec![b' '; MAX_PLAN_HINT]).unwrap();
    manifest.plan_sha256 = "0".repeat(64);
    manifest.plan = PathBuf::from("bloated-plan.json");
    let path = base.join("bloated.json");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let error = decode::read_manifest(&path).unwrap_err();
    assert!(error.contains("plan exceeds 1 MiB bound"), "{error}");
}

const MAX_PLAN_HINT: usize = 1024 * 1024 + 1;
