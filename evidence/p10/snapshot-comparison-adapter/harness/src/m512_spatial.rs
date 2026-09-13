use crate::model::{Evolution, Manifest, Snapshot, TimeDiagnosticOutput};

const R6_SOURCE: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
const N512_SOURCE: &str = "e25f3816f83c6a7c07202cac2878f58ace460511";
const N512_TEST_SOURCE: &str = "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645";
const R6_PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
const N512_PROFILE: &str = "n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f";
const R6_PLAN: &str = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
const N512_PLAN: &str = "4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8";
const CASE_SHA256: &str = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";

pub(crate) fn diagnostic_output<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    crate::compare::diagnostic_output(
        left_manifest,
        left,
        right_manifest,
        right,
        admitted_bytes,
        "p10-snapshot-matched-m512-spatial-diagnostic-output-v1",
        "MATCHED_M512_SPATIAL_DIAGNOSTIC",
        None,
    )
}

pub(crate) fn validate_manifest_pair(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.dimensions != [384; 3]
        || right.dimensions != [512; 3]
        || left.source_commit != R6_SOURCE
        || right.source_commit != N512_SOURCE
        || left.plan_sha256 != R6_PLAN
        || right.plan_sha256 != N512_PLAN
        || profile(left) != Some(R6_PROFILE)
        || profile(right) != Some(N512_PROFILE)
        || !identity_field_equals(&left.identity, "source", R6_SOURCE)
        || !identity_field_equals(&right.identity, "source", N512_SOURCE)
        || !identity_field_equals(
            &right.identity,
            "production_source",
            "0843b8b18e6a096a0208e3d896e391c7b1b2f5e0",
        )
        || !identity_field_equals(&right.identity, "test_source", N512_TEST_SOURCE)
        || !identity_field_equals(
            &right.identity,
            "external_stop",
            "pgid-watchdog-v3-confirmed-identity-absolute-deadline",
        )
        || !identity_field_equals(&right.identity, "schema", "p10-avx-n512-observer-state-v1")
        || left.evolution != right.evolution
        || !is_r6_evolution(&left.evolution)
        || !is_r6_guard(left)
        || !is_r6_guard(right)
    {
        return Err("matched M512 spatial diagnostic contract mismatch".into());
    }
    crate::compare::validate_fixed_diagnostic_pair(left, right)
}

fn profile(manifest: &Manifest) -> Option<&str> {
    manifest
        .profile
        .as_ref()
        .map(|profile| profile.value.as_str())
}

fn identity_field_equals(identity: &str, key: &str, expected: &str) -> bool {
    let mut values = identity
        .split(';')
        .filter_map(|field| field.split_once('='));
    values
        .by_ref()
        .filter(|(candidate, _)| *candidate == key)
        .map(|(_, value)| value)
        .eq([expected])
}

fn is_r6_evolution(evolution: &Evolution) -> bool {
    evolution.case_sha256 == CASE_SHA256
        && evolution.quantum_exponent == -20
        && evolution.clock_target == 8192
        && evolution.comparison_endpoint == 4096
        && evolution.lengths == [1.0; 3]
        && evolution.viscosity.to_bits() == 1.0_f64.to_bits()
        && evolution.method == "cox-matthews"
        && evolution.integration_force_dimensions == [512; 3]
        && evolution.schedule
            == [
                crate::model::ScheduleSegment {
                    from_inclusive: 0,
                    until_exclusive: 2048,
                    step_ticks: 64,
                },
                crate::model::ScheduleSegment {
                    from_inclusive: 2048,
                    until_exclusive: 4096,
                    step_ticks: 128,
                },
            ]
        && evolution.absolute_tolerances == [1e-5, 1e-4]
        && evolution.relative_tolerances == [1e-5, 1e-5]
}

fn is_r6_guard(manifest: &Manifest) -> bool {
    manifest.admission_guard.as_ref().is_some_and(|guard| {
        guard.advective_limit.to_bits() == 3.3_f64.to_bits() && guard.maximum_attempts == 48
    })
}
