//! Separately bound contract for the conditional N256/M512 vs N384/M512
//! endpoint spatial pair at clock 4096. This contract never weakens and never
//! overlaps the reviewed N384->N512 `MATCHED_M512_SPATIAL_DIAGNOSTIC` contract
//! in the p10 snapshot-comparison-adapter.
use crate::model::{AdmissionGuard, Evolution, Manifest, ScheduleSegment};

pub(crate) const R6_SOURCE: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
pub(crate) const R6_PLAN: &str = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
pub(crate) const R6_PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
pub(crate) const R6_ENDPOINT_COEFFICIENT_SHA256: &str =
    "1d1500409962c4af37182f6733c2c88a086247c8e8728570e9c93238ed1c04fd";
pub(crate) const R6_ENDPOINT_FILE_SHA256: &str =
    "43308ca8ddb499916a09183266e3a83811b3214536c14c7ba2f6814363cef0a0";

pub(crate) const COARSE_PROFILE: &str =
    "n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c-user-scope-v1";
pub(crate) const COARSE_HOST: &str = "baccus";
pub(crate) const COARSE_EXTERNAL_STOP: &str =
    "systemd-user-scope-gnu-timeout-term-killafter2s-memorymax103079215104-tasksmax256-v1";
pub(crate) const COARSE_NUMA: &str = "unbound-all-visible-cpus-cgroup-memory-limit";
pub(crate) const COARSE_STATE_SCHEMA: &str = "p10-avx-n256-m512-observer-state-v1";
pub(crate) const COARSE_ATTEMPT_SCHEMA: &str = "p10-avx-scheduled-attempt-v3";
pub(crate) const COARSE_W3_SOURCE: &str = "f13c29c9ae91d0b8cf7a790132deb9bd076911c0";
pub(crate) const COARSE_RHS_W3: &str = "layout384-width3-bidirectional-add2734010240";
pub(crate) const COARSE_FORCE_W3: &str = "layout512-width3-forward-add4318465792";

pub(crate) const CASE_SHA256: &str =
    "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";

pub(crate) const COARSE_DIMENSIONS: [usize; 3] = [256; 3];
pub(crate) const FINE_DIMENSIONS: [usize; 3] = [384; 3];
pub(crate) const FORCE_DIMENSIONS: [usize; 3] = [512; 3];
pub(crate) const QUANTUM_EXPONENT: i32 = -20;
pub(crate) const CLOCK_TARGET: u128 = 8192;
pub(crate) const COMPARISON_ENDPOINT: u128 = 4096;
pub(crate) const MAXIMUM_ATTEMPTS: u128 = 48;
pub(crate) const ENDPOINT_EPOCH: u128 = 48;
pub(crate) const ENDPOINT_ACCEPTED_STEPS: u128 = 48;

/// Full-state bytes for one N256 half-spectrum snapshot (256^2*129*3*16).
pub(crate) const COARSE_STATE_BYTES: usize = 405_798_912;
/// Full-state bytes for one N384 half-spectrum snapshot (384^2*193*3*16).
pub(crate) const FINE_STATE_BYTES: usize = 1_366_032_384;

const IDENTITY_FORBIDDEN_MARKERS: [&str; 7] = [
    "host=sulaco",
    "pgid-watchdog-v2",
    "n384",
    "n512",
    "parallel8",
    "p10-avx-n384",
    "p10-avx-n512",
];

pub(crate) fn validate_manifest_pair(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.dimensions != COARSE_DIMENSIONS
        || right.dimensions != FINE_DIMENSIONS
        || left.evolution != right.evolution
        || !is_closed_pair_evolution(&left.evolution)
        || left.elapsed != right.elapsed
        || left.target != right.target
        || left.elapsed != COMPARISON_ENDPOINT
        || left.target != CLOCK_TARGET
        || left.epoch != ENDPOINT_EPOCH
        || right.epoch != ENDPOINT_EPOCH
        || left.accepted_steps != ENDPOINT_ACCEPTED_STEPS
        || right.accepted_steps != ENDPOINT_ACCEPTED_STEPS
        || schedule_steps(&left.evolution)? != ENDPOINT_ACCEPTED_STEPS
        || !is_closed_guard(&left.admission_guard)
        || !is_closed_guard(&right.admission_guard)
    {
        return Err("n256-m512 vs n384-m512 pair contract mismatch".into());
    }
    validate_fine_side(right)?;
    validate_coarse_side(left)
}

pub(crate) fn is_closed_pair_evolution(evolution: &Evolution) -> bool {
    is_closed_case(evolution) && is_closed_flow(evolution) && is_closed_lattice(evolution)
}

fn is_closed_case(evolution: &Evolution) -> bool {
    evolution.case_sha256 == CASE_SHA256
        && evolution.quantum_exponent == QUANTUM_EXPONENT
        && evolution.clock_target == CLOCK_TARGET
        && evolution.comparison_endpoint == COMPARISON_ENDPOINT
}

fn is_closed_flow(evolution: &Evolution) -> bool {
    evolution.lengths == [1.0; 3]
        && evolution.viscosity.to_bits() == 1.0_f64.to_bits()
        && evolution.method == "cox-matthews"
        && evolution.absolute_tolerances == [1e-5, 1e-4]
        && evolution.relative_tolerances == [1e-5, 1e-5]
}

fn is_closed_lattice(evolution: &Evolution) -> bool {
    evolution.integration_force_dimensions == FORCE_DIMENSIONS
        && evolution.schedule
            == [
                ScheduleSegment {
                    from_inclusive: 0,
                    until_exclusive: 2048,
                    step_ticks: 64,
                },
                ScheduleSegment {
                    from_inclusive: 2048,
                    until_exclusive: 4096,
                    step_ticks: 128,
                },
            ]
}

pub(crate) fn is_closed_guard(guard: &AdmissionGuard) -> bool {
    guard.advective_limit.to_bits() == 3.3_f64.to_bits()
        && guard.maximum_attempts == MAXIMUM_ATTEMPTS
}

fn validate_fine_side(right: &Manifest) -> Result<(), String> {
    if right.source_commit != R6_SOURCE
        || right.plan_sha256 != R6_PLAN
        || right.profile.value != R6_PROFILE
        || !right.profile.matches_identity(&right.identity)
        || right.identity != crate::fine_identity::FINE_IDENTITY
        || right.backend != crate::fine_identity::FINE_BACKEND
        || right.execution != crate::fine_identity::FINE_EXECUTION
        || right.coefficient_sha256 != R6_ENDPOINT_COEFFICIENT_SHA256
        || right.file_sha256 != R6_ENDPOINT_FILE_SHA256
    {
        return Err("n384/m512 fine-side lineage binding mismatch".into());
    }
    Ok(())
}

fn validate_coarse_side(left: &Manifest) -> Result<(), String> {
    if left.profile.value != COARSE_PROFILE
        || !left.profile.matches_identity(&left.identity)
        || left.lineage.is_none()
        || IDENTITY_FORBIDDEN_MARKERS
            .into_iter()
            .any(|marker| left.identity.contains(marker))
    {
        return Err("n256/m512 coarse-side lineage binding mismatch".into());
    }
    for (key, expected) in [
        ("source", left.source_commit.as_str()),
        ("profile", COARSE_PROFILE),
        ("case", CASE_SHA256),
        ("retained", "256"),
        ("force_samples", "512"),
        ("rhs_dealias", "384"),
        ("rhs_w3", COARSE_RHS_W3),
        ("force_w3", COARSE_FORCE_W3),
        ("w3_source", COARSE_W3_SOURCE),
        ("schema", COARSE_STATE_SCHEMA),
        ("attempt_schema", COARSE_ATTEMPT_SCHEMA),
        ("host", COARSE_HOST),
        ("numa", COARSE_NUMA),
        ("external_stop", COARSE_EXTERNAL_STOP),
        ("endpoint", "4096"),
        ("advective_limit", "3.3"),
        ("resume", "unsupported"),
    ] {
        if !identity_field_equals(&left.identity, key, expected) {
            return Err("n256/m512 coarse-side lineage binding mismatch".into());
        }
    }
    Ok(())
}

/// Field must appear exactly once with the exact expected value.
pub(crate) fn identity_field_equals(identity: &str, key: &str, expected: &str) -> bool {
    let mut values = identity
        .split(';')
        .filter_map(|field| field.split_once('='));
    values
        .by_ref()
        .filter(|(candidate, _)| *candidate == key)
        .map(|(_, value)| value)
        .eq([expected])
}

/// Exact schedule-derived accepted-step count for a closed segment list.
pub(crate) fn schedule_steps(evolution: &Evolution) -> Result<u128, String> {
    let mut next = 0_u128;
    let mut sum = 0_u128;
    for segment in &evolution.schedule {
        if segment.from_inclusive != next
            || segment.until_exclusive <= segment.from_inclusive
            || segment.step_ticks == 0
            || !(segment.until_exclusive - segment.from_inclusive)
                .is_multiple_of(segment.step_ticks)
        {
            return Err("invalid pair schedule".into());
        }
        sum = sum
            .checked_add((segment.until_exclusive - segment.from_inclusive) / segment.step_ticks)
            .ok_or_else(|| "pair schedule step count overflow".to_string())?;
        next = segment.until_exclusive;
    }
    if next != evolution.comparison_endpoint {
        return Err("pair schedule does not reach endpoint".into());
    }
    Ok(sum)
}

/// Accepted-step count and therefore epoch after committing the state at the
/// given positive clock of the closed h64-through-2048 / h128-through-4096
/// schedule.
pub(crate) fn steps_through(clock: u128) -> Result<u128, String> {
    if clock == 0 || clock > COMPARISON_ENDPOINT {
        return Err("lineage clock outside the closed schedule".into());
    }
    Ok(if clock <= 2048 {
        if !clock.is_multiple_of(64) {
            return Err("lineage clock off the h64 schedule".into());
        }
        clock / 64
    } else {
        if !clock.is_multiple_of(128) {
            return Err("lineage clock off the h128 schedule".into());
        }
        32 + (clock - 2048) / 128
    })
}

/// The exact 48 positive committed clocks the N256/M512 lineage must supply.
pub(crate) fn required_clocks() -> Vec<u128> {
    (64..=2048)
        .step_by(64)
        .chain((2176..=COMPARISON_ENDPOINT).step_by(128))
        .collect()
}
