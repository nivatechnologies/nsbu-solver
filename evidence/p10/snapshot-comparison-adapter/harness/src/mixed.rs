//! Closed three-state force/space sensitivity diagnostic.

pub(crate) use crate::mixed_math::calculate;
use crate::{
    mixed_math::MixedMetrics,
    model::{
        AcceptanceOutput, AdmissionGuard, ComparisonKind, Evolution, Hashes, Manifest,
        ProfileBinding, Snapshot,
    },
};
use nsbu_solver::Complex64;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct SideOutput<'a> {
    pub role: &'static str,
    pub dimensions: [usize; 3],
    pub evolution: &'a Evolution,
    pub identity: &'a str,
    pub profile: &'a ProfileBinding,
    pub admission_guard: &'a AdmissionGuard,
    pub backend: &'a str,
    pub execution: &'a str,
    pub source_commit: &'a str,
    pub plan_sha256: &'a str,
    pub hashes: Hashes<'a>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct MixedClock {
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct MixedDiagnosticOutput<'a> {
    pub schema: &'static str,
    pub comparison_kind: &'static str,
    pub acceptance: AcceptanceOutput,
    pub coarse: SideOutput<'a>,
    pub baseline: SideOutput<'a>,
    pub force: SideOutput<'a>,
    pub clock: MixedClock,
    pub metrics: MixedMetrics,
    pub admitted_bytes: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct BoundSide<'a> {
    pub manifest: &'a Manifest,
    pub snapshot: &'a Snapshot,
}

pub(crate) fn diagnostic_output<'a>(
    coarse_manifest: &'a Manifest,
    coarse: &'a Snapshot,
    baseline_manifest: &'a Manifest,
    baseline: &'a Snapshot,
    force_manifest: &'a Manifest,
    force: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<MixedDiagnosticOutput<'a>, String> {
    validate_bound_inputs(
        coarse_manifest,
        coarse,
        baseline_manifest,
        baseline,
        force_manifest,
        force,
    )?;
    let metrics = calculate_bound_states(
        coarse_manifest,
        coarse,
        baseline_manifest,
        baseline,
        force_manifest,
        force,
    )?;
    Ok(bound_output(
        BoundSide {
            manifest: coarse_manifest,
            snapshot: coarse,
        },
        BoundSide {
            manifest: baseline_manifest,
            snapshot: baseline,
        },
        BoundSide {
            manifest: force_manifest,
            snapshot: force,
        },
        metrics,
        admitted_bytes,
    ))
}

fn validate_bound_inputs(
    coarse_manifest: &Manifest,
    coarse: &Snapshot,
    baseline_manifest: &Manifest,
    baseline: &Snapshot,
    force_manifest: &Manifest,
    force: &Snapshot,
) -> Result<(), String> {
    validate_manifests(coarse_manifest, baseline_manifest, force_manifest)?;
    if coarse.clock != baseline.clock || baseline.clock != force.clock {
        return Err("mixed diagnostic snapshot clock mismatch".into());
    }
    Ok(())
}

fn calculate_bound_states(
    coarse_manifest: &Manifest,
    coarse: &Snapshot,
    baseline_manifest: &Manifest,
    baseline: &Snapshot,
    force_manifest: &Manifest,
    force: &Snapshot,
) -> Result<MixedMetrics, String> {
    calculate(
        coarse_manifest.domain()?,
        arrays(coarse),
        baseline_manifest.domain()?,
        arrays(baseline),
        force_manifest.domain()?,
        arrays(force),
    )
}

pub(crate) fn bound_output<'a>(
    coarse: BoundSide<'a>,
    baseline: BoundSide<'a>,
    force: BoundSide<'a>,
    metrics: MixedMetrics,
    admitted_bytes: usize,
) -> MixedDiagnosticOutput<'a> {
    MixedDiagnosticOutput {
        schema: "p10-snapshot-mixed-force-space-diagnostic-output-v1",
        comparison_kind: "MIXED_FORCE_SPACE_DIAGNOSTIC",
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        coarse: side("coarse_u256_m384", coarse.manifest, coarse.snapshot),
        baseline: side("baseline_u384_m384", baseline.manifest, baseline.snapshot),
        force: side("force_u384_m512", force.manifest, force.snapshot),
        clock: MixedClock {
            elapsed: coarse.snapshot.clock.elapsed,
            target: coarse.snapshot.clock.target,
            epoch: coarse.snapshot.clock.epoch,
            accepted_steps: coarse.snapshot.clock.accepted_steps,
        },
        metrics,
        admitted_bytes,
    }
}

pub(crate) fn validate_manifests(
    coarse: &Manifest,
    baseline: &Manifest,
    force: &Manifest,
) -> Result<(), String> {
    if [
        coarse.comparison_kind,
        baseline.comparison_kind,
        force.comparison_kind,
    ] != [ComparisonKind::MixedForceSpaceDiagnostic; 3]
        || coarse.dimensions != [256; 3]
        || baseline.dimensions != [384; 3]
        || force.dimensions != [384; 3]
        || coarse.evolution.integration_force_dimensions != [384; 3]
        || baseline.evolution.integration_force_dimensions != [384; 3]
        || force.evolution.integration_force_dimensions != [512; 3]
        || coarse.evolution != baseline.evolution
        || !same_except_force(&baseline.evolution, &force.evolution)
    {
        return Err("mixed force/space diagnostic contract mismatch".into());
    }
    if manifest_clock(coarse) != manifest_clock(baseline)
        || manifest_clock(baseline) != manifest_clock(force)
    {
        return Err("mixed force/space diagnostic clock mismatch".into());
    }
    validate_side(coarse)?;
    validate_side(baseline)?;
    validate_side(force)
}

fn validate_side(manifest: &Manifest) -> Result<(), String> {
    let profile = manifest.profile.as_ref().ok_or("missing exact profile")?;
    let guard = manifest
        .admission_guard
        .as_ref()
        .ok_or("missing admission guard metadata")?;
    if manifest.arithmetic_control.is_some()
        || !profile.matches_identity(&manifest.identity)
        || manifest.epoch != manifest.accepted_steps
        || manifest.accepted_steps > guard.maximum_attempts
        || schedule_steps(&manifest.evolution)? != manifest.accepted_steps
    {
        return Err("mixed diagnostic manifest binding mismatch".into());
    }
    Ok(())
}

fn same_except_force(left: &Evolution, right: &Evolution) -> bool {
    left.case_sha256 == right.case_sha256
        && left.quantum_exponent == right.quantum_exponent
        && left.clock_target == right.clock_target
        && left.comparison_endpoint == right.comparison_endpoint
        && bits(left.lengths, right.lengths)
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.method == right.method
        && left.schedule == right.schedule
        && bits(left.absolute_tolerances, right.absolute_tolerances)
        && bits(left.relative_tolerances, right.relative_tolerances)
}

fn bits<const N: usize>(left: [f64; N], right: [f64; N]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn schedule_steps(evolution: &Evolution) -> Result<u128, String> {
    evolution.schedule.iter().try_fold(0_u128, |sum, segment| {
        let span = segment
            .until_exclusive
            .checked_sub(segment.from_inclusive)
            .filter(|_| segment.step_ticks != 0)
            .ok_or_else(|| "invalid mixed diagnostic schedule".to_string())?;
        sum.checked_add(span / segment.step_ticks)
            .ok_or_else(|| "mixed diagnostic schedule step count overflow".into())
    })
}

fn manifest_clock(manifest: &Manifest) -> [u128; 4] {
    [
        manifest.elapsed,
        manifest.target,
        manifest.epoch,
        manifest.accepted_steps,
    ]
}

fn arrays(snapshot: &Snapshot) -> [&[Complex64]; 3] {
    std::array::from_fn(|axis| snapshot.coefficients[axis].as_slice())
}

fn side<'a>(role: &'static str, manifest: &'a Manifest, snapshot: &'a Snapshot) -> SideOutput<'a> {
    SideOutput {
        role,
        dimensions: manifest.dimensions,
        evolution: &manifest.evolution,
        identity: &manifest.identity,
        profile: manifest.profile.as_ref().unwrap(),
        admission_guard: manifest.admission_guard.as_ref().unwrap(),
        backend: &manifest.backend,
        execution: &manifest.execution,
        source_commit: &manifest.source_commit,
        plan_sha256: &manifest.plan_sha256,
        hashes: Hashes {
            coefficient_sha256: &snapshot.coefficient_sha256,
            file_sha256: &snapshot.file_sha256,
        },
    }
}
