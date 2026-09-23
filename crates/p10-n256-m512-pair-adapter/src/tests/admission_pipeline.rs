use super::*;
use crate::run;
use std::ffi::OsString;

/// A complete synthetic capture (real metadata-only rest + full-length states)
/// plus the reviewed-style receipt bound to its hashes, so pipeline ordering and
/// the resource cap can be exercised independently of the absent real lineage.
fn staged_pair(dir: &std::path::Path) -> (Manifest, Manifest, LineageIntake) {
    let source = "a".repeat(40);
    let plan_path = dir.join("coarse-frozen-plan.json");
    fs::write(&plan_path, b"{\"synthetic\":true}\n").unwrap();
    let plan_sha256 = format!("{:x}", Sha256::digest(b"{\"synthetic\":true}\n"));
    let (intake_path, intake) = intake_fixture(dir, &source, &plan_sha256);
    let mut left = coarse_manifest(dir, &source);
    left.plan_sha256 = plan_sha256;
    bind_intake(&mut left, &intake_path);
    left.coefficient_sha256 = intake.states[47].coefficient_sha256.clone();
    left.file_sha256 = intake.states[47].file_sha256.clone();
    left.snapshot = intake.states[47].path.clone();
    for record in &intake.states {
        let steps = contract::steps_through(record.clock).unwrap();
        write_sparse_state(
            &record.path,
            &left.identity,
            ClockHeader {
                elapsed: record.clock,
                target: contract::CLOCK_TARGET,
                epoch: steps,
                accepted_steps: steps,
            },
        );
    }
    (left, fine_manifest(dir), intake)
}

fn deadline_far_future() -> u64 {
    pair::now_epoch().unwrap() + 24 * 60 * 60
}

#[test]
fn admitted_pipeline_reports_expected_reservation() {
    let dir = root("pipeline-admit");
    let (left, right, intake) = staged_pair(&dir);
    let anchor = trusted_anchor(&intake);
    let admitted = pair::admit_with_anchor(
        left,
        right,
        4 * 1024 * 1024 * 1024,
        deadline_far_future(),
        pair::now_epoch().unwrap(),
        Some(&anchor),
    )
    .expect("anchor-authenticated lineage admits to the reservation stage");
    assert_eq!(
        admitted.admitted_bytes(),
        contract::COARSE_STATE_BYTES + contract::FINE_STATE_BYTES + 1024 * 1024
    );
}

#[test]
fn cap_exhaustion_refuses_before_any_load() {
    let dir = root("pipeline-cap");
    let (left, right, intake) = staged_pair(&dir);
    let anchor = trusted_anchor(&intake);
    let error = pair::admit_with_anchor(
        left,
        right,
        contract::COARSE_STATE_BYTES,
        deadline_far_future(),
        pair::now_epoch().unwrap(),
        Some(&anchor),
    )
    .unwrap_err();
    assert!(error.contains("exceeds cap"), "{error}");
}

#[test]
fn elapsed_deadline_refuses_before_contract_or_lineage_reads() {
    let dir = root("pipeline-deadline");
    let mut left = coarse_manifest(&dir, &"a".repeat(40));
    left.plan_sha256 = "5".repeat(64);
    let right = fine_manifest(&dir);
    let error = pair::admit(left, right, usize::MAX, 1, 2).unwrap_err();
    assert!(error.contains("deadline elapsed"), "{error}");
}

#[test]
fn contract_violation_refuses_before_lineage_absence() {
    let dir = root("pipeline-order-contract");
    let (mut left, right, _) = staged_pair(&dir);
    left.dimensions = [320; 3];
    let error = pair::admit(
        left,
        right,
        usize::MAX,
        deadline_far_future(),
        pair::now_epoch().unwrap(),
    )
    .unwrap_err();
    assert!(error.contains("pair contract mismatch"), "{error}");
}

#[test]
fn lineage_absence_refuses_before_cap_evaluation() {
    let dir = root("pipeline-order-lineage");
    let source = "a".repeat(40);
    fs::write(
        dir.join("coarse-frozen-plan.json"),
        b"{\"synthetic\":true}\n",
    )
    .unwrap();
    let plan_sha256 = format!("{:x}", Sha256::digest(b"{\"synthetic\":true}\n"));
    let (intake_path, _) = intake_fixture(&dir, &source, &plan_sha256);
    let mut left = coarse_manifest(&dir, &source);
    left.plan_sha256 = plan_sha256;
    bind_intake(&mut left, &intake_path);
    let right = fine_manifest(&dir);
    let error = pair::admit(
        left,
        right,
        usize::MAX,
        deadline_far_future(),
        pair::now_epoch().unwrap(),
    )
    .unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
}

#[test]
fn coarse_manifest_endpoint_hash_must_match_intake_before_load() {
    let dir = root("pipeline-hash-order");
    let (mut left, right, _) = staged_pair(&dir);
    left.coefficient_sha256 = "7".repeat(64);
    // Contract only pins fine hashes; the intake endpoint binding must refuse
    // before the completed-lineage (unverified) stage is even considered.
    let error = pair::admit(
        left,
        right,
        usize::MAX,
        deadline_far_future(),
        pair::now_epoch().unwrap(),
    )
    .unwrap_err();
    assert!(error.contains("endpoint hashes do not match"), "{error}");
}

#[test]
fn cli_admit_reports_lineage_absent_for_a_missing_intake() {
    let dir = root("cli-absent");
    let error = run(&[
        OsString::from("admit"),
        dir.join("lineage-intake.json").into_os_string(),
    ])
    .unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
}

#[test]
fn cli_admit_refuses_a_self_attested_synthetic_lineage() {
    // The review's reproduce: a complete self-attested intake (present states and
    // rest) is metadata, not a completed independent trajectory, and no trusted
    // closed receipt exists at runtime, so `admit` must refuse, never "admit".
    let dir = root("cli-admit");
    let (left, _, _) = staged_pair(&dir);
    let intake_path = left.lineage.as_ref().unwrap().intake.clone();
    let error = run(&[OsString::from("admit"), intake_path.into_os_string()]).unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_UNVERIFIED), "{error}");
    assert!(!error.contains("admitted"), "{error}");
}

#[test]
fn cli_compare_reports_lineage_absent_when_no_coarse_capture_exists() {
    let missing =
        OsString::from("/mnt/niva-array/nsbu-solver/work/does-not-exist-n256-m512-coarse.json");
    let fine_path = {
        let dir = root("cli-compare-fine");
        let right = fine_manifest(&dir);
        let path = dir.join("fine.json");
        fs::write(&path, serde_json::to_vec(&right).unwrap()).unwrap();
        path
    };
    let error = run(&[
        OsString::from("compare"),
        missing,
        fine_path.into_os_string(),
        OsString::from("1800000000"),
        OsString::from(format!("{}", pair::now_epoch().unwrap() + 3600)),
    ])
    .unwrap_err();
    assert!(error.starts_with(lineage::LINEAGE_ABSENT), "{error}");
    assert!(error.contains("prepared_not_executed"), "{error}");
}

#[test]
fn cli_rejects_malformed_invocations() {
    assert!(run(&[]).is_err());
    assert!(run(&[OsString::from("admit")]).is_err());
    assert!(run(&[
        OsString::from("compare"),
        OsString::from("a"),
        OsString::from("b"),
        OsString::from("not-a-cap"),
        OsString::from("1"),
    ])
    .is_err());
    assert!(run(&[
        OsString::from("compare"),
        OsString::from("a"),
        OsString::from("b"),
        OsString::from("1"),
        OsString::from("not-an-epoch"),
    ])
    .is_err());
}
