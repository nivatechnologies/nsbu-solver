//! End-to-end decoder admission of byte-exact producer output: the fixture
//! below is the publication of
//! `tools/prepare_temporal_comparison.py` over a synthetic concrete family
//! plan, replayed beside the reviewed v3 launch plan (h64 side) or a pending
//! placeholder (h32 side) and co-located arithmetic evidence, admitted only
//! through `decode::read_manifest`, without ever opening the bound (absent)
//! payload. The synthetic identities and epoch records demonstrate decoder
//! admission only — never a real capture, comparison or window result.

use super::*;

const H64_CLOCK0512: &[u8] = include_bytes!("m512-temporal/h64h32-clock0512-h64.json");
const H32_CLOCK0512: &[u8] = include_bytes!("m512-temporal/h64h32-clock0512-h32.json");
const V3_LAUNCH_PLAN: &[u8] = include_bytes!(
    "../../../../n512-m512-endpoint-prep-20260913/proposed-launch/v3-launch-plan.json"
);

fn stage(base: &std::path::Path, bytes: &[u8], plan: Option<&[u8]>, rebind: bool) -> PathBuf {
    let dir = base.join("a/b/c/d");
    fs::create_dir_all(&dir).unwrap();
    let manifest_path = dir.join("input.json");
    let parsed: Manifest = serde_json::from_slice(bytes).unwrap();
    let control = parsed.arithmetic_control.clone().unwrap();
    let mut evidence = serde_json::to_vec_pretty(&control.review).unwrap();
    evidence.push(b'\n');
    let digest = format!("{:x}", Sha256::digest(&evidence));
    if rebind {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value["arithmetic_control"]["evidence_sha256"] = serde_json::Value::String(digest);
        fs::write(&manifest_path, serde_json::to_vec(&value).unwrap()).unwrap();
    } else {
        assert_eq!(digest, control.evidence_sha256);
        fs::write(&manifest_path, bytes).unwrap();
    }
    fs::write(dir.join(&control.evidence), &evidence).unwrap();
    if let Some(plan) = plan {
        let relative = parsed.plan.strip_prefix("../../../../").unwrap();
        let staged = base.join(relative);
        fs::create_dir_all(staged.parent().unwrap()).unwrap();
        fs::write(&staged, plan).unwrap();
    }
    manifest_path
}

#[test]
fn producer_h64_manifest_admits_through_read_manifest_with_co_located_evidence() {
    let base = root("temporal-e2e-h64");
    let path = stage(&base, H64_CLOCK0512, Some(V3_LAUNCH_PLAN), false);
    let manifest = decode::read_manifest(&path).unwrap();
    assert_eq!(manifest.comparison_kind, ComparisonKind::TimeDiagnostic);
    assert_eq!(manifest.dimensions, [512; 3]);
    assert_eq!(manifest.evolution.integration_force_dimensions, [512; 3]);
    assert_eq!(manifest.evolution.comparison_endpoint, 512);
    let control = manifest.arithmetic_control.clone().unwrap();
    assert_eq!(
        control.evidence_sha256,
        "ac7b05041a96d493b2b3e8838192494976f64709e0c40dc1dfc81185cf14ff97"
    );
    assert_eq!(control.review.integration_force_dimensions, [512; 3]);
    assert!(manifest.snapshot.ends_with("state.bin"));
    assert!(!manifest.snapshot.exists());
    assert_eq!(decode::state_bytes(&manifest).unwrap(), 3_233_808_384);
    assert!(decode::load(&manifest).is_err());
}

#[test]
fn producer_h32_manifest_passes_admission_before_its_pending_plan_file() {
    let base = root("temporal-e2e-h32");
    let path = stage(
        &base,
        H32_CLOCK0512,
        Some(b"pending capture plan bytes\n"),
        false,
    );
    let error = decode::read_manifest(&path).unwrap_err();
    assert!(error.contains("frozen plan SHA-256 mismatch"), "{error}");
}

#[test]
fn read_manifest_refuses_wrong_m_temporal_admissions() {
    let base = root("temporal-e2e-wrong-m");
    let mut manifest: Manifest = serde_json::from_slice(H64_CLOCK0512).unwrap();
    manifest.evolution.integration_force_dimensions = [640; 3];
    let path = base.join("evolution-640.json");
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(decode::read_manifest(&path)
        .unwrap_err()
        .contains("invalid evolution semantics"));
    let mut value: serde_json::Value = serde_json::from_slice(H64_CLOCK0512).unwrap();
    value["arithmetic_control"]["review"]["integration_force_dimensions"] =
        serde_json::json!([384, 384, 384]);
    let path = stage(
        &base,
        &serde_json::to_vec(&value).unwrap(),
        Some(V3_LAUNCH_PLAN),
        true,
    );
    assert!(decode::read_manifest(&path)
        .unwrap_err()
        .contains("arithmetic-control binding"));
}

#[test]
fn m384_time_manifest_path_still_admits_with_m384_arithmetic_review() {
    let base = root("temporal-e2e-m384");
    let mut left = manifest(base.join("left.bin"), 4, "left-h32");
    let mut right = manifest(base.join("right.bin"), 4, "right-piecewise");
    enable_time(&mut left, &mut right, &base);
    let values = fields(left.domain().unwrap().layout(), 1.0);
    write(&mut left, &values);
    write(&mut right, &values);
    for (name, item) in [("left.json", &mut left), ("right.json", &mut right)] {
        item.snapshot = PathBuf::from(item.snapshot.file_name().unwrap());
        item.plan = PathBuf::from(item.plan.file_name().unwrap());
        let path = base.join(name);
        fs::write(&path, serde_json::to_vec(item).unwrap()).unwrap();
        let read = decode::read_manifest(&path).unwrap();
        assert_eq!(read.comparison_kind, ComparisonKind::TimeDiagnostic);
        assert_eq!(read.evolution.integration_force_dimensions, [384; 3]);
    }
}
