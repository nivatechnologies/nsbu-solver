use super::*;

const PAIR_MISMATCH: &str = "n256-m512 vs n384-m512 pair contract mismatch";
const FINE_MISMATCH: &str = "n384/m512 fine-side lineage binding mismatch";
const COARSE_MISMATCH: &str = "n256/m512 coarse-side lineage binding mismatch";

#[test]
fn state_byte_constants_match_the_solver_layout() {
    assert_eq!(
        decode::state_bytes(&coarse_manifest(&root("bytes"), &"a".repeat(40))).unwrap(),
        contract::COARSE_STATE_BYTES
    );
    assert_eq!(
        decode::state_bytes(&fine_manifest(&root("bytes"))).unwrap(),
        contract::FINE_STATE_BYTES
    );
}

#[test]
fn closed_pair_contract_admits() {
    let dir = root("admit");
    let (left, right) = valid_pair(&dir);
    assert_eq!(contract::validate_manifest_pair(&left, &right), Ok(()));
}

#[test]
fn contract_refuses_reversed_pair_direction() {
    let dir = root("reverse");
    let (left, right) = valid_pair(&dir);
    assert_eq!(
        contract::validate_manifest_pair(&right, &left).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_dimensions() {
    let dir = root("dimensions");
    let (mut left, right) = valid_pair(&dir);
    left.dimensions = [320; 3];
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.dimensions = [512; 3];
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, right) = valid_pair(&dir);
    left.dimensions = [384; 3];
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_force_grid() {
    for grid in [[384, 384, 384], [512, 512, 511], [768, 768, 768]] {
        let (mut left, mut right) = valid_pair(&root("force-grid"));
        left.evolution.integration_force_dimensions = grid;
        right.evolution.integration_force_dimensions = grid;
        assert_eq!(
            contract::validate_manifest_pair(&left, &right).unwrap_err(),
            PAIR_MISMATCH
        );
    }
}

#[test]
fn contract_refuses_wrong_clocks_on_either_side() {
    let (mut left, mut right) = valid_pair(&root("clocks"));
    left.elapsed = 4095;
    right.elapsed = 4095;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, mut right) = valid_pair(&root("clocks"));
    left.target = 8191;
    right.target = 8191;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, right) = valid_pair(&root("clocks"));
    left.epoch = 47;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (left, mut right) = valid_pair(&root("clocks"));
    right.accepted_steps = 47;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, right) = valid_pair(&root("clocks"));
    left.elapsed = 2048;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_source_and_plan_on_fine_side() {
    let dir = root("source");
    let (left, mut right) = valid_pair(&dir);
    right.source_commit = "f".repeat(40);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.plan_sha256 = "e".repeat(64);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_endpoint_state_hashes_on_fine_side() {
    let dir = root("fine-hashes");
    let (left, mut right) = valid_pair(&dir);
    right.coefficient_sha256 = "9".repeat(64);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.file_sha256 = "8".repeat(64);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
}

#[test]
fn contract_refuses_fine_identity_backend_or_execution_drift() {
    let dir = root("fine-binding");
    let (left, mut right) = valid_pair(&dir);
    right.identity = format!("{};extra=1", right.identity);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.backend = "other-backend".into();
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.execution = "other-execution".into();
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_profile_on_either_side() {
    let dir = root("profile");
    let (mut left, right) = valid_pair(&dir);
    left.profile.value = "n256-m512-tampered".into();
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        COARSE_MISMATCH
    );
    let (mut left, right) = valid_pair(&dir);
    left.identity = left.identity.replace(
        &format!("profile={}", contract::COARSE_PROFILE),
        "profile=other;profile-other-tail",
    );
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        COARSE_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.profile.value = "n384-m512-tampered".into();
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        FINE_MISMATCH
    );
}

#[test]
fn contract_refuses_wrong_rest_proximate_identity_on_coarse_side() {
    let dir = root("coarse-identity");
    for (key, tampered) in [
        ("host=baccus", "host=sulaco"),
        (
            "external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline",
            "external_stop=pgid-watchdog-v2-starttime-cmdline-deadline",
        ),
        ("retained=256", "retained=384"),
        ("force_samples=512", "force_samples=384"),
        (
            "schema=p10-avx-n256-m512-observer-state-v1",
            "schema=p10-avx-n512-observer-state-v1",
        ),
        (
            &format!("rhs_w3={}", contract::COARSE_RHS_W3),
            "rhs_w3=layout576-width3-bidirectional-add9200779136",
        ),
        ("resume=unsupported", "resume=supported"),
    ] {
        let (mut left, right) = valid_pair(&dir);
        assert!(left.identity.contains(key), "fixture lacks {key}");
        left.identity = left.identity.replace(key, tampered);
        let error = contract::validate_manifest_pair(&left, &right).unwrap_err();
        assert_eq!(error, COARSE_MISMATCH, "for {key} -> {tampered}");
    }
}

#[test]
fn contract_refuses_duplicate_coarse_identity_keys() {
    let dir = root("duplicate-keys");
    let (mut left, right) = valid_pair(&dir);
    left.identity = format!("{};retained=256", left.identity);
    assert_eq!(
        left.identity
            .split(';')
            .filter(|field| field.starts_with("retained="))
            .count(),
        2
    );
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        COARSE_MISMATCH
    );
}

#[test]
fn contract_refuses_missing_coarse_lineage_binding() {
    let dir = root("missing-lineage");
    let (mut left, right) = valid_pair(&dir);
    left.lineage = None;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        COARSE_MISMATCH
    );
}

#[test]
fn contract_refuses_one_sided_evolution_drift() {
    let dir = root("evolution");
    let (mut left, right) = valid_pair(&dir);
    left.evolution.viscosity = 0.5;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, mut right) = valid_pair(&dir);
    left.evolution.viscosity = 0.5;
    right.evolution.viscosity = 0.5;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, mut right) = valid_pair(&dir);
    left.evolution.case_sha256 = "d".repeat(64);
    right.evolution.case_sha256 = "d".repeat(64);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (mut left, mut right) = valid_pair(&dir);
    left.evolution.quantum_exponent = -21;
    right.evolution.quantum_exponent = -21;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn contract_refuses_guard_deviation_on_either_side() {
    let dir = root("guard");
    let (mut left, right) = valid_pair(&dir);
    left.admission_guard.maximum_attempts = 49;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
    let (left, mut right) = valid_pair(&dir);
    right.admission_guard.advective_limit = 3.4;
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn contract_refuses_case_mismatch_between_sides() {
    let dir = root("case-mismatch");
    let (mut left, right) = valid_pair(&dir);
    left.evolution.case_sha256 = "b".repeat(64);
    assert_eq!(
        contract::validate_manifest_pair(&left, &right).unwrap_err(),
        PAIR_MISMATCH
    );
}

#[test]
fn required_clocks_match_the_closed_schedule_exactly() {
    let clocks = contract::required_clocks();
    assert_eq!(clocks.len(), 48);
    assert_eq!(clocks[0], 64);
    assert_eq!(clocks[31], 2048);
    assert_eq!(clocks[32], 2176);
    assert_eq!(clocks[47], 4096);
    for (index, clock) in clocks.iter().enumerate() {
        assert_eq!(
            contract::steps_through(*clock).unwrap(),
            u128::try_from(index).unwrap() + 1
        );
    }
    assert!(contract::steps_through(0).is_err());
    assert!(contract::steps_through(4097).is_err());
    assert!(contract::steps_through(2112).is_err());
}
