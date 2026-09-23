//! Admission and endpoint-diagnostic pipeline for the N256/M512 vs N384/M512
//! clock-4096 spatial pair.
//!
//! Order is deliberate: deadline, then manifest-level pair contract, then the
//! completed-lineage gate (refuses runtime admission while the N256/M512
//! lineage is absent), then the resource cap against bounded metadata, and
//! only then full state loads and the comparison itself. The result is an
//! endpoint spatial diagnostic, never a converged-PDE-window acceptance: the
//! output carries `acceptance.status = "not_assessed"`, `accepted_windows = 0`
//! and `interpretation.converged_pde_window = false`.
use crate::absolute;
use crate::contract;
use crate::decode;
use crate::lineage::{self, Intake};
use crate::model::{
    debug, AcceptanceOutput, ClockOutput, DiagnosticOutput, Hashes, InterpretationOutput, Manifest,
    NormOutput, Snapshot,
};
use nsbu_solver::{diagnostics::comparison::ComparisonPlan, domain::Domain, Complex64};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const OUTPUT_SCHEMA: &str = "p10-n256-m512-pair-endpoint-diagnostic-output-v1";
pub(crate) const COMPARISON_KIND: &str = "N256_M512_PAIR_ENDPOINT_DIAGNOSTIC";
pub(crate) const LINEAGE_STATUS_BOUND: &str = "completed_lineage_intake_bound";

#[derive(Debug)]
pub(crate) struct Admitted {
    left: Manifest,
    right: Manifest,
    intake: Intake,
    admitted_bytes: usize,
}

impl Admitted {
    #[cfg(test)]
    pub(crate) fn admitted_bytes(&self) -> usize {
        self.admitted_bytes
    }
}

/// Everything admitted except the full-state loads. `now_epoch` is injectable
/// so the deadline path is deterministic under test. Runtime admission uses the
/// compiled-in reviewed closed-lineage receipt, which is absent for N256/M512,
/// so completed-lineage admission stays refused where evidence is absent.
pub(crate) fn admit(
    left: Manifest,
    right: Manifest,
    cap_bytes: usize,
    deadline_epoch: u64,
    now_epoch: u64,
) -> Result<Admitted, String> {
    admit_with_anchor(
        left,
        right,
        cap_bytes,
        deadline_epoch,
        now_epoch,
        lineage::reviewed_anchor().as_ref(),
    )
}

/// The pipeline with an explicit closed-lineage receipt. Runtime callers pass
/// `reviewed_anchor()` (currently `None`); tests pass a synthetic receipt bound
/// to fixture hashes to exercise ordering/cap independently of the absent
/// real N256 lineage.
pub(crate) fn admit_with_anchor(
    left: Manifest,
    right: Manifest,
    cap_bytes: usize,
    deadline_epoch: u64,
    now_epoch: u64,
    anchor: Option<&lineage::TrustedLineage>,
) -> Result<Admitted, String> {
    if now_epoch >= deadline_epoch {
        return Err(format!(
            "pair comparison deadline elapsed: epoch {now_epoch} is not before {deadline_epoch}"
        ));
    }
    contract::validate_manifest_pair(&left, &right)?;
    if decode::state_bytes(&left)? != contract::COARSE_STATE_BYTES
        || decode::state_bytes(&right)? != contract::FINE_STATE_BYTES
    {
        return Err("pair state geometry does not match the bound lineages".into());
    }
    let intake = match anchor {
        Some(anchor) => lineage::admit_through_manifest_with_anchor(&left, Some(anchor))?,
        None => lineage::admit_through_manifest(&left)?,
    };
    let admitted_bytes = decode::preflight(&left, &right)?;
    if admitted_bytes > cap_bytes {
        return Err(format!(
            "comparison reservation {admitted_bytes} exceeds cap {cap_bytes}"
        ));
    }
    Ok(Admitted {
        left,
        right,
        intake,
        admitted_bytes,
    })
}

pub(crate) fn run(admitted: Admitted) -> Result<String, String> {
    let left = decode::load(&admitted.left)?;
    let right = decode::load(&admitted.right)?;
    let output = endpoint_diagnostic(
        &admitted.left,
        &left,
        &admitted.right,
        &right,
        admitted.admitted_bytes,
        admitted.intake.value.states.len(),
    )?;
    serde_json::to_string_pretty(&output).map_err(debug)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn endpoint_diagnostic<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
    coarse_lineage_states: usize,
) -> Result<DiagnosticOutput<'a>, String> {
    if left.clock != right.clock || left.clock.elapsed != contract::COMPARISON_ENDPOINT {
        return Err("endpoint clock mismatch".into());
    }
    let metrics = compare_core(
        left_manifest.domain()?,
        right_manifest.domain()?,
        std::array::from_fn(|axis| left.coefficients[axis].as_slice()),
        std::array::from_fn(|axis| right.coefficients[axis].as_slice()),
    )?;
    Ok(DiagnosticOutput {
        schema: OUTPUT_SCHEMA,
        comparison_kind: COMPARISON_KIND,
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        interpretation: InterpretationOutput {
            kind: "endpoint_spatial_pair_diagnostic",
            endpoint_clock: contract::COMPARISON_ENDPOINT,
            endpoint_physical_time: "1/256",
            quantum: "2^-20",
            endpoint_only: true,
            converged_pde_window: false,
            window_qualification: "not_assessed",
            refinement_model_note: "single pair only; the 256-to-384 difference is D1 of an unequal-ratio (3/2, 4/3) model and is not an observed refinement ratio or order",
        },
        lineage_status: LINEAGE_STATUS_BOUND,
        coarse_lineage_states,
        left_evolution: &left_manifest.evolution,
        right_evolution: &right_manifest.evolution,
        left_identity: &left_manifest.identity,
        left_profile: &left_manifest.profile,
        left_source_commit: &left_manifest.source_commit,
        left_plan_sha256: &left_manifest.plan_sha256,
        left_lineage_intake_sha256: left_manifest
            .lineage
            .as_ref()
            .ok_or("missing lineage binding")?
            .intake_sha256
            .clone(),
        right_identity: &right_manifest.identity,
        right_profile: &right_manifest.profile,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
        left_hashes: hashes(left),
        right_hashes: hashes(right),
        clock: ClockOutput {
            elapsed: left.clock.elapsed,
            target: left.clock.target,
            coarse_epoch: left.clock.epoch,
            fine_epoch: right.clock.epoch,
            coarse_accepted_steps: left.clock.accepted_steps,
            fine_accepted_steps: right.clock.accepted_steps,
        },
        full: metrics.full,
        common: metrics.common,
        newly_resolved: metrics.newly_resolved,
        fine_absolute: metrics.fine_absolute,
        mean_error: metrics.mean_error,
        admitted_bytes,
    })
}

#[derive(Debug)]
pub(crate) struct CoreMetrics {
    pub(crate) full: NormOutput,
    pub(crate) common: NormOutput,
    pub(crate) newly_resolved: NormOutput,
    pub(crate) fine_absolute: NormOutput,
    pub(crate) mean_error: [f64; 3],
}

/// Pure band comparison over borrowed spectra plus the fine absolute norms.
pub(crate) fn compare_core(
    coarse: Domain,
    fine: Domain,
    coarse_values: [&[Complex64]; 3],
    fine_values: [&[Complex64]; 3],
) -> Result<CoreMetrics, String> {
    let plan = ComparisonPlan::new(coarse, fine).map_err(debug)?;
    let result = plan.compare(coarse_values, fine_values).map_err(debug)?;
    let fine_absolute = absolute::measure(fine, fine_values)?;
    Ok(CoreMetrics {
        full: result.full.into(),
        common: result.common.into(),
        newly_resolved: result.newly_resolved.into(),
        fine_absolute,
        mean_error: result.mean_error,
    })
}

fn hashes(snapshot: &Snapshot) -> Hashes<'_> {
    Hashes {
        coefficient_sha256: &snapshot.coefficient_sha256,
        file_sha256: &snapshot.file_sha256,
    }
}

pub(crate) fn now_epoch() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(debug)
}
