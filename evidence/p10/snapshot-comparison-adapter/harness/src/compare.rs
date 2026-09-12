use crate::model::{ClockOutput, Hashes, Manifest, Output, Snapshot};
use nsbu_solver::diagnostics::comparison::ComparisonPlan;

pub(crate) fn compare<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<Output<'a>, String> {
    if left_manifest.evolution != right_manifest.evolution {
        return Err("evolution semantics mismatch".into());
    }
    if left.clock.elapsed != right.clock.elapsed
        || left.clock.target != right.clock.target
        || left.clock.epoch != right.clock.epoch
        || left.clock.accepted_steps != right.clock.accepted_steps
    {
        return Err("endpoint clock mismatch".into());
    }
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
        full: result.full.into(),
        common: result.common.into(),
        newly_resolved: result.newly_resolved.into(),
        fine_absolute,
        mean_error: result.mean_error,
        admitted_bytes,
    })
}

fn hashes(snapshot: &Snapshot) -> Hashes<'_> {
    Hashes {
        coefficient_sha256: &snapshot.coefficient_sha256,
        file_sha256: &snapshot.file_sha256,
    }
}
