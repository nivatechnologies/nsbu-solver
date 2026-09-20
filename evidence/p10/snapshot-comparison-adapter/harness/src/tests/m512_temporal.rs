//! Producer-to-consumer metadata admission for the exact N512/M512 temporal
//! family: the fixture manifests below are the byte-exact output of
//! `tools/prepare_temporal_comparison.py` over a synthetic concrete family
//! plan with M512 evidence materialized beside every manifest. The h32/h16
//! identities, plans and epoch records are synthetic, so these fixtures
//! demonstrate metadata decoding and manifest admission only: the checks
//! below inspect decoded manifest metadata and refusal branches, no
//! numerical comparator trajectory is executed, and no real capture,
//! comparison or window result is claimed.

use super::*;

const H64_CLOCK0512: &str = include_str!("m512-temporal/h64h32-clock0512-h64.json");
const H32_CLOCK0512: &str = include_str!("m512-temporal/h64h32-clock0512-h32.json");
const H32_CLOCK2560: &str = include_str!("m512-temporal/h32h16-clock2560-h32.json");
const H16_CLOCK2560: &str = include_str!("m512-temporal/h32h16-clock2560-h16.json");

fn pair(left: &str, right: &str) -> (Manifest, Manifest) {
    (
        serde_json::from_str(left).unwrap(),
        serde_json::from_str(right).unwrap(),
    )
}

fn ticks(manifest: &Manifest) -> Vec<u128> {
    manifest
        .evolution
        .schedule
        .iter()
        .map(|segment| segment.step_ticks)
        .collect()
}

#[test]
fn producer_emitted_m512_temporal_manifests_are_admitted() {
    for (left, right) in [
        pair(H64_CLOCK0512, H32_CLOCK0512),
        pair(H32_CLOCK2560, H16_CLOCK2560),
    ] {
        assert_eq!(left.comparison_kind, ComparisonKind::TimeDiagnostic);
        assert_eq!(left.dimensions, [512; 3]);
        assert_eq!(left.evolution.integration_force_dimensions, [512; 3]);
        compare::validate_manifest_pair(&left, &right).unwrap();
    }
    let (h64, h32) = pair(H64_CLOCK0512, H32_CLOCK0512);
    assert_eq!(ticks(&h64), vec![64]);
    assert_eq!(ticks(&h32), vec![32]);
    let (h32, h16) = pair(H32_CLOCK2560, H16_CLOCK2560);
    assert_eq!(ticks(&h32), vec![32, 64]);
    assert_eq!(ticks(&h16), vec![16, 32]);
    assert_eq!(h32.epoch, 72);
    assert_eq!(h16.epoch, 144);
    assert_eq!(h32.elapsed, 2560);
}

#[test]
fn m512_temporal_manifests_refuse_wrong_force_dimensions() {
    let (mut left, mut right) = pair(H64_CLOCK0512, H32_CLOCK0512);
    for side in [&mut left, &mut right] {
        side.evolution.integration_force_dimensions = [640; 3];
    }
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("force dimensions are not admitted"));
    let (mut left, mut right) = pair(H64_CLOCK0512, H32_CLOCK0512);
    for side in [&mut left, &mut right] {
        side.dimensions = [384; 3];
    }
    assert!(compare::validate_manifest_pair(&left, &right)
        .unwrap_err()
        .contains("force dimensions are not admitted"));
    let (mut left, right) = pair(H64_CLOCK0512, H32_CLOCK0512);
    left.evolution.integration_force_dimensions = [384; 3];
    assert!(compare::validate_manifest_pair(&left, &right).is_err());
}

#[test]
fn m384_time_family_pair_is_still_admitted() {
    let dir = root("m384-time-preserved");
    let mut left = manifest(dir.join("left.bin"), 4, "left-h32");
    let mut right = manifest(dir.join("right.bin"), 4, "right-piecewise");
    right.source_commit = "b".repeat(40);
    right.backend = "fixture-w3-backend".into();
    right.execution = "fixture-w3-execution".into();
    enable_time(&mut left, &mut right, &dir);
    compare::validate_manifest_pair(&left, &right).unwrap();
}
