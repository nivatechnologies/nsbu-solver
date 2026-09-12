use crate::model::{
    AcceptanceOutput, AdmissionGuard, ClockOutput, ComparisonKind, Evolution, Hashes, Manifest,
    Output, Snapshot, TimeClockOutput, TimeDiagnosticOutput,
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
        right_identity: &right_manifest.identity,
        right_backend: &right_manifest.backend,
        right_execution: &right_manifest.execution,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
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
    if left.clock != crate::model::ClockHeader::from_manifest(left_manifest)
        || right.clock != crate::model::ClockHeader::from_manifest(right_manifest)
    {
        return Err("time-diagnostic snapshot clock mismatch".into());
    }
    let metrics = calculate(left_manifest, left, right_manifest, right)?;
    Ok(TimeDiagnosticOutput {
        schema: "p10-snapshot-time-diagnostic-output-v1",
        comparison_kind: "TIME_DIAGNOSTIC",
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        left_evolution: &left_manifest.evolution,
        right_evolution: &right_manifest.evolution,
        left_identity: &left_manifest.identity,
        left_profile: left_manifest.profile.as_deref().unwrap(),
        left_admission_guard: left_manifest.admission_guard.as_ref().unwrap(),
        left_backend: &left_manifest.backend,
        left_execution: &left_manifest.execution,
        left_source_commit: &left_manifest.source_commit,
        left_plan_sha256: &left_manifest.plan_sha256,
        right_identity: &right_manifest.identity,
        right_profile: right_manifest.profile.as_deref().unwrap(),
        right_admission_guard: right_manifest.admission_guard.as_ref().unwrap(),
        right_backend: &right_manifest.backend,
        right_execution: &right_manifest.execution,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
        arithmetic_control: left_manifest.arithmetic_control.as_ref().unwrap(),
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
            if !same_guard(
                left_manifest.admission_guard.as_ref(),
                right_manifest.admission_guard.as_ref(),
            ) {
                return Err("admission guard mismatch".into());
            }
            Ok(())
        }
        (ComparisonKind::TimeDiagnostic, ComparisonKind::TimeDiagnostic) => {
            validate_time_manifests(left_manifest, right_manifest)
        }
        _ => Err("comparison kind mismatch".into()),
    }
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
    validate_time_side(left_manifest, true)?;
    validate_time_side(right_manifest, false)?;
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
    Ok(())
}

fn validate_time_side(manifest: &Manifest, left: bool) -> Result<(), String> {
    let steps = schedule_steps(&manifest.evolution)?;
    let guard = manifest
        .admission_guard
        .as_ref()
        .ok_or("missing admission guard metadata")?;
    if !guard.advective_limit.is_finite()
        || guard.advective_limit <= 0.0
        || manifest.accepted_steps != steps
        || manifest.epoch != steps
        || guard.maximum_attempts != steps
    {
        return Err("time-diagnostic schedule/header derivation mismatch".into());
    }
    let profile = manifest.profile.as_deref().ok_or("missing exact profile")?;
    if profile.is_empty() || identity_profile(&manifest.identity) != Some(profile) {
        return Err("exact profile does not match snapshot identity".into());
    }
    let control = manifest
        .arithmetic_control
        .as_ref()
        .ok_or("missing arithmetic-control binding")?;
    let side = if left { &control.left } else { &control.right };
    if side.source_commit != manifest.source_commit
        || side.backend != manifest.backend
        || side.execution != manifest.execution
        || side.profile != profile
    {
        return Err("arithmetic-control side binding mismatch".into());
    }
    Ok(())
}

fn identity_profile(identity: &str) -> Option<&str> {
    identity
        .split(';')
        .find_map(|field| field.strip_prefix("profile="))
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

fn same_f64_array<const N: usize>(left: [f64; N], right: [f64; N]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn same_guard(left: Option<&AdmissionGuard>, right: Option<&AdmissionGuard>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.advective_limit.to_bits() == right.advective_limit.to_bits()
                && left.maximum_attempts == right.maximum_attempts
        }
        _ => false,
    }
}

fn hashes(snapshot: &Snapshot) -> Hashes<'_> {
    Hashes {
        coefficient_sha256: &snapshot.coefficient_sha256,
        file_sha256: &snapshot.file_sha256,
    }
}
