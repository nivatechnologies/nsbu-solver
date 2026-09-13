use crate::model::{
    AcceptanceOutput, ClockOutput, ComparisonKind, Evolution, Hashes, Manifest, Output, Snapshot,
    TimeClockOutput, TimeDiagnosticOutput,
};
use nsbu_solver::diagnostics::comparison::ComparisonPlan;

pub(crate) fn compare<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<Output<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    if left.clock.elapsed != right.clock.elapsed
        || left.clock.target != right.clock.target
        || left.clock.epoch != right.clock.epoch
        || left.clock.accepted_steps != right.clock.accepted_steps
    {
        return Err("endpoint clock mismatch".into());
    }
    let metrics = calculate(left_manifest, left, right_manifest, right)?;
    Ok(Output {
        schema: "p10-snapshot-comparison-output-v1",
        evolution: &left_manifest.evolution,
        left_identity: &left_manifest.identity,
        left_backend: &left_manifest.backend,
        left_execution: &left_manifest.execution,
        left_source_commit: &left_manifest.source_commit,
        left_plan_sha256: &left_manifest.plan_sha256,
        left_admission_guard: left_manifest.admission_guard.as_ref(),
        right_identity: &right_manifest.identity,
        right_backend: &right_manifest.backend,
        right_execution: &right_manifest.execution,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
        right_admission_guard: right_manifest.admission_guard.as_ref(),
        left_hashes: hashes(left),
        right_hashes: hashes(right),
        clock: ClockOutput {
            elapsed: left.clock.elapsed,
            target: left.clock.target,
            epoch: left.clock.epoch,
            left_accepted_steps: left.clock.accepted_steps,
            right_accepted_steps: right.clock.accepted_steps,
        },
        full: metrics.full,
        common: metrics.common,
        newly_resolved: metrics.newly_resolved,
        fine_absolute: metrics.fine_absolute,
        mean_error: metrics.mean_error,
        admitted_bytes,
    })
}

pub(crate) fn time_diagnostic<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    diagnostic_output(
        left_manifest,
        left,
        right_manifest,
        right,
        admitted_bytes,
        "p10-snapshot-time-diagnostic-output-v1",
        "TIME_DIAGNOSTIC",
        left_manifest.arithmetic_control.as_ref(),
    )
}

pub(crate) fn force_resolution_diagnostic<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    diagnostic_output(
        left_manifest,
        left,
        right_manifest,
        right,
        admitted_bytes,
        "p10-snapshot-force-resolution-diagnostic-output-v1",
        "FORCE_RESOLUTION_DIAGNOSTIC",
        None,
    )
}

pub(crate) fn matched_m512_spatial_diagnostic<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    diagnostic_output(
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

pub(crate) fn method_diagnostic<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    diagnostic_output(
        left_manifest,
        left,
        right_manifest,
        right,
        admitted_bytes,
        "p10-snapshot-method-diagnostic-output-v1",
        "METHOD_DIAGNOSTIC",
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn diagnostic_output<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
    schema: &'static str,
    comparison_kind: &'static str,
    arithmetic_control: Option<&'a crate::model::ArithmeticControl>,
) -> Result<TimeDiagnosticOutput<'a>, String> {
    if left.clock != crate::model::ClockHeader::from_manifest(left_manifest)
        || right.clock != crate::model::ClockHeader::from_manifest(right_manifest)
    {
        return Err("diagnostic snapshot clock mismatch".into());
    }
    let metrics = calculate(left_manifest, left, right_manifest, right)?;
    Ok(TimeDiagnosticOutput {
        schema,
        comparison_kind,
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        left_evolution: &left_manifest.evolution,
        right_evolution: &right_manifest.evolution,
        left_identity: &left_manifest.identity,
        left_profile: left_manifest.profile.as_ref().unwrap(),
        left_admission_guard: left_manifest.admission_guard.as_ref().unwrap(),
        left_backend: &left_manifest.backend,
        left_execution: &left_manifest.execution,
        left_source_commit: &left_manifest.source_commit,
        left_plan_sha256: &left_manifest.plan_sha256,
        right_identity: &right_manifest.identity,
        right_profile: right_manifest.profile.as_ref().unwrap(),
        right_admission_guard: right_manifest.admission_guard.as_ref().unwrap(),
        right_backend: &right_manifest.backend,
        right_execution: &right_manifest.execution,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
        arithmetic_control,
        left_hashes: hashes(left),
        right_hashes: hashes(right),
        clock: TimeClockOutput {
            elapsed: left.clock.elapsed,
            target: left.clock.target,
            left_epoch: left.clock.epoch,
            right_epoch: right.clock.epoch,
            left_accepted_steps: left.clock.accepted_steps,
            right_accepted_steps: right.clock.accepted_steps,
        },
        full: metrics.full,
        common: metrics.common,
        newly_resolved: metrics.newly_resolved,
        fine_absolute: metrics.fine_absolute,
        mean_error: metrics.mean_error,
        admitted_bytes,
    })
}

struct Metrics {
    full: crate::model::NormOutput,
    common: crate::model::NormOutput,
    newly_resolved: crate::model::NormOutput,
    fine_absolute: crate::model::NormOutput,
    mean_error: [f64; 3],
}

fn calculate(
    left_manifest: &Manifest,
    left: &Snapshot,
    right_manifest: &Manifest,
    right: &Snapshot,
) -> Result<Metrics, String> {
    let plan = ComparisonPlan::new(left_manifest.domain()?, right_manifest.domain()?)
        .map_err(crate::model::debug)?;
    let result = plan
        .compare(
            std::array::from_fn(|axis| left.coefficients[axis].as_slice()),
            std::array::from_fn(|axis| right.coefficients[axis].as_slice()),
        )
        .map_err(crate::model::debug)?;
    let fine_absolute = crate::absolute::measure(
        right_manifest.domain()?,
        std::array::from_fn(|axis| right.coefficients[axis].as_slice()),
    )?;
    Ok(Metrics {
        full: result.full.into(),
        common: result.common.into(),
        newly_resolved: result.newly_resolved.into(),
        fine_absolute,
        mean_error: result.mean_error,
    })
}

pub(crate) fn validate_manifest_pair(
    left_manifest: &Manifest,
    right_manifest: &Manifest,
) -> Result<(), String> {
    match (
        left_manifest.comparison_kind,
        right_manifest.comparison_kind,
    ) {
        (ComparisonKind::MatchedSpatial, ComparisonKind::MatchedSpatial) => {
            if left_manifest.evolution != right_manifest.evolution {
                return Err("evolution semantics mismatch".into());
            }
            Ok(())
        }
        (
            ComparisonKind::MatchedM512SpatialDiagnostic,
            ComparisonKind::MatchedM512SpatialDiagnostic,
        ) => validate_m512_spatial_manifests(left_manifest, right_manifest),
        (ComparisonKind::TimeDiagnostic, ComparisonKind::TimeDiagnostic) => {
            validate_time_manifests(left_manifest, right_manifest)
        }
        (ComparisonKind::ForceResolutionDiagnostic, ComparisonKind::ForceResolutionDiagnostic) => {
            validate_force_manifests(left_manifest, right_manifest)
        }
        (ComparisonKind::MethodDiagnostic, ComparisonKind::MethodDiagnostic) => {
            validate_method_manifests(left_manifest, right_manifest)
        }
        (ComparisonKind::MixedForceSpaceDiagnostic, ComparisonKind::MixedForceSpaceDiagnostic) => {
            Err("mixed force/space diagnostic requires three inputs".into())
        }
        _ => Err("comparison kind mismatch".into()),
    }
}

const R6_M512_SOURCE: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
const N512_M512_SOURCE: &str = "9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645";
const R6_M512_PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
const N512_M512_PROFILE: &str = "n512-m512-h64to2048-h128to4096-cadv33-w3-9eba11f";
const R6_M512_PLAN: &str = "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
const N512_M512_PLAN: &str = "6e8103a1937e3e31be8b147a936b843d4dc166877ef5de5540fb51429e672634";
const CASE_SHA256: &str = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";

fn validate_m512_spatial_manifests(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.dimensions != [384; 3]
        || right.dimensions != [512; 3]
        || left.source_commit != R6_M512_SOURCE
        || right.source_commit != N512_M512_SOURCE
        || left.plan_sha256 != R6_M512_PLAN
        || right.plan_sha256 != N512_M512_PLAN
        || left.profile.as_ref().map(|profile| profile.value.as_str()) != Some(R6_M512_PROFILE)
        || right.profile.as_ref().map(|profile| profile.value.as_str()) != Some(N512_M512_PROFILE)
        || !identity_field_equals(&left.identity, "source", R6_M512_SOURCE)
        || !identity_field_equals(&right.identity, "source", N512_M512_SOURCE)
        || !identity_field_equals(
            &right.identity,
            "production_source",
            "0843b8b18e6a096a0208e3d896e391c7b1b2f5e0",
        )
        || !identity_field_equals(&right.identity, "test_source", N512_M512_SOURCE)
        || !identity_field_equals(
            &right.identity,
            "external_stop",
            "pgid-watchdog-v2-starttime-cmdline-deadline",
        )
        || !identity_field_equals(&right.identity, "schema", "p10-avx-n512-observer-state-v1")
        || left.evolution.integration_force_dimensions != [512; 3]
        || left.evolution != right.evolution
        || !is_r6_m512_evolution(&left.evolution)
        || !is_r6_guard(left)
        || !is_r6_guard(right)
    {
        return Err("matched M512 spatial diagnostic contract mismatch".into());
    }
    validate_fixed_diagnostic_pair(left, right)
}

fn identity_field_equals(identity: &str, key: &str, expected: &str) -> bool {
    identity
        .split(';')
        .any(|field| field.split_once('=') == Some((key, expected)))
}

fn is_r6_m512_evolution(evolution: &Evolution) -> bool {
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

fn validate_force_manifests(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.dimensions != [384; 3]
        || right.dimensions != [384; 3]
        || left.evolution.integration_force_dimensions != [384; 3]
        || right.evolution.integration_force_dimensions != [512; 3]
        || !same_evolution_except_force(&left.evolution, &right.evolution)
    {
        return Err("force-resolution diagnostic contract mismatch".into());
    }
    validate_fixed_diagnostic_pair(left, right)
}

fn validate_method_manifests(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.dimensions != [384; 3]
        || right.dimensions != [384; 3]
        || left.evolution.method != "cox-matthews"
        || right.evolution.method != "hochbruck-ostermann"
        || left.evolution.integration_force_dimensions != [384; 3]
        || right.evolution.integration_force_dimensions != [384; 3]
        || !same_evolution_except_method(&left.evolution, &right.evolution)
    {
        return Err("method diagnostic contract mismatch".into());
    }
    validate_fixed_diagnostic_pair(left, right)
}

fn validate_fixed_diagnostic_pair(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.elapsed != right.elapsed || left.target != right.target {
        return Err("diagnostic physical clock mismatch".into());
    }
    if left.arithmetic_control.is_some() || right.arithmetic_control.is_some() {
        return Err("diagnostic does not admit an arithmetic-control override".into());
    }
    validate_schedule_bound_side(left)?;
    validate_schedule_bound_side(right)
}

fn validate_time_manifests(
    left_manifest: &Manifest,
    right_manifest: &Manifest,
) -> Result<(), String> {
    if left_manifest.dimensions != right_manifest.dimensions
        || !same_evolution_except_schedule(&left_manifest.evolution, &right_manifest.evolution)
    {
        return Err("time-diagnostic immutable semantics mismatch".into());
    }
    if left_manifest.elapsed != right_manifest.elapsed
        || left_manifest.target != right_manifest.target
    {
        return Err("time-diagnostic physical clock mismatch".into());
    }
    validate_schedule_bound_side(left_manifest)?;
    validate_schedule_bound_side(right_manifest)?;
    let left_control = left_manifest
        .arithmetic_control
        .as_ref()
        .ok_or("missing arithmetic-control binding")?;
    let right_control = right_manifest
        .arithmetic_control
        .as_ref()
        .ok_or("missing arithmetic-control binding")?;
    if left_control != right_control {
        return Err("arithmetic-control binding mismatch".into());
    }
    let review = &left_control.review;
    if review.case_sha256 != left_manifest.evolution.case_sha256
        || review.method != left_manifest.evolution.method
        || review.integration_force_dimensions
            != left_manifest.evolution.integration_force_dimensions
    {
        return Err("arithmetic-control physical contract mismatch".into());
    }
    validate_time_lineage(left_manifest, true)?;
    validate_time_lineage(right_manifest, false)?;
    Ok(())
}

fn validate_schedule_bound_side(manifest: &Manifest) -> Result<(), String> {
    let steps = schedule_steps(&manifest.evolution)?;
    let guard = manifest
        .admission_guard
        .as_ref()
        .ok_or("missing admission guard metadata")?;
    if !guard.advective_limit.is_finite()
        || guard.advective_limit <= 0.0
        || manifest.accepted_steps != steps
        || manifest.epoch != steps
        || steps > guard.maximum_attempts
    {
        return Err("diagnostic schedule/header derivation mismatch".into());
    }
    let profile = manifest.profile.as_ref().ok_or("missing exact profile")?;
    if !profile.matches_identity(&manifest.identity) {
        return Err("exact profile does not match snapshot identity".into());
    }
    Ok(())
}

fn validate_time_lineage(manifest: &Manifest, left: bool) -> Result<(), String> {
    let profile = manifest.profile.as_ref().ok_or("missing exact profile")?;
    let control = manifest
        .arithmetic_control
        .as_ref()
        .ok_or("missing arithmetic-control binding")?;
    let side = if left {
        &control.review.reviewed_lineage.left
    } else {
        &control.review.reviewed_lineage.right
    };
    if side.source_commit != manifest.source_commit
        || side.backend != manifest.backend
        || side.execution != manifest.execution
        || side.profile != *profile
    {
        return Err("arithmetic-control side binding mismatch".into());
    }
    Ok(())
}

fn schedule_steps(evolution: &Evolution) -> Result<u128, String> {
    let mut next = 0_u128;
    let mut sum = 0_u128;
    for segment in &evolution.schedule {
        if segment.from_inclusive != next
            || segment.until_exclusive <= segment.from_inclusive
            || segment.step_ticks == 0
        {
            return Err("invalid time-diagnostic schedule".into());
        }
        let span = segment.until_exclusive - segment.from_inclusive;
        if !span.is_multiple_of(segment.step_ticks) {
            return Err("invalid time-diagnostic schedule".into());
        }
        let steps = (segment.until_exclusive - segment.from_inclusive) / segment.step_ticks;
        sum = sum
            .checked_add(steps)
            .ok_or_else(|| "time-diagnostic step count overflow".to_string())?;
        next = segment.until_exclusive;
    }
    if next != evolution.comparison_endpoint {
        return Err("invalid time-diagnostic schedule".into());
    }
    Ok(sum)
}

fn same_evolution_except_schedule(left: &Evolution, right: &Evolution) -> bool {
    left.case_sha256 == right.case_sha256
        && left.quantum_exponent == right.quantum_exponent
        && left.clock_target == right.clock_target
        && left.comparison_endpoint == right.comparison_endpoint
        && same_f64_array(left.lengths, right.lengths)
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.method == right.method
        && left.integration_force_dimensions == right.integration_force_dimensions
        && same_f64_array(left.absolute_tolerances, right.absolute_tolerances)
        && same_f64_array(left.relative_tolerances, right.relative_tolerances)
}

fn same_evolution_except_force(left: &Evolution, right: &Evolution) -> bool {
    left.case_sha256 == right.case_sha256
        && left.quantum_exponent == right.quantum_exponent
        && left.clock_target == right.clock_target
        && left.comparison_endpoint == right.comparison_endpoint
        && same_f64_array(left.lengths, right.lengths)
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.method == right.method
        && left.schedule == right.schedule
        && same_f64_array(left.absolute_tolerances, right.absolute_tolerances)
        && same_f64_array(left.relative_tolerances, right.relative_tolerances)
}

fn same_evolution_except_method(left: &Evolution, right: &Evolution) -> bool {
    left.case_sha256 == right.case_sha256
        && left.quantum_exponent == right.quantum_exponent
        && left.clock_target == right.clock_target
        && left.comparison_endpoint == right.comparison_endpoint
        && same_f64_array(left.lengths, right.lengths)
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.integration_force_dimensions == right.integration_force_dimensions
        && left.schedule == right.schedule
        && same_f64_array(left.absolute_tolerances, right.absolute_tolerances)
        && same_f64_array(left.relative_tolerances, right.relative_tolerances)
}

fn same_f64_array<const N: usize>(left: [f64; N], right: [f64; N]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn hashes(snapshot: &Snapshot) -> Hashes<'_> {
    Hashes {
        coefficient_sha256: &snapshot.coefficient_sha256,
        file_sha256: &snapshot.file_sha256,
    }
}
