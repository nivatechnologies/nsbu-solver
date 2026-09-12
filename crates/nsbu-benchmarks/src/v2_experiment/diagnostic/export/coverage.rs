use super::{json::Json, values as v, DiagnosticExportError, DiagnosticExportPlan};
use crate::{
    regions::{CoverageStatus, NominalRegion, RegionCoverage},
    v2_experiment::{coverage::CoverageFamilySample, diagnostic::DiagnosticEvent},
};
use std::io::Write;

pub fn validate(
    plan: DiagnosticExportPlan,
    event: DiagnosticEvent,
    sample: CoverageFamilySample,
) -> Result<(), DiagnosticExportError> {
    let accepted = event
        .accepted()
        .sample()
        .ok_or(DiagnosticExportError::InvalidReport)?;
    if sample.clock() != event.clock()
        || sample.family_identity() != plan.family_identity
        || sample.tracking_identity() != accepted.regional_reference.identity()
        || sample.sampling().layout != plan.settings.reference_samples
        || sample.sampling().points != plan.settings.reference_samples.real_len()
        || sample.sampling().relative_floors.map(f64::to_bits)
            != plan.settings.reference_floors.map(f64::to_bits)
    {
        return Err(DiagnosticExportError::InvalidReport);
    }
    let metadata = crate::v2_experiment::coverage::sampling_metadata(
        accepted.regional_reference,
        event.clock(),
        plan.settings.reference_floors,
    )
    .map_err(|_| DiagnosticExportError::InvalidReport)?;
    if sample.sampling() != metadata {
        return Err(DiagnosticExportError::InvalidReport);
    }
    validate_regions(
        sample.core(),
        plan.settings.coverage_panels,
        event.clock(),
        NominalRegion::CORE,
    )?;
    validate_regions(
        sample.annulus(),
        plan.settings.coverage_panels,
        event.clock(),
        NominalRegion::ANNULUS,
    )
}

fn validate_regions(
    values: [RegionCoverage; 3],
    panels: [usize; 3],
    clock: nsbu_solver::domain::TickClock,
    region: NominalRegion,
) -> Result<(), DiagnosticExportError> {
    let status = crate::regions::coverage_status_at(clock, region)
        .map_err(|_| DiagnosticExportError::InvalidReport)?;
    for (value, coarse) in values.into_iter().zip(panels) {
        if value.status != status
            || value.panels
                != coarse
                    .checked_mul(2)
                    .ok_or(DiagnosticExportError::SizeOverflow)?
            || value.evaluations
                != coarse
                    .checked_mul(3)
                    .and_then(|n| n.checked_add(2))
                    .ok_or(DiagnosticExportError::SizeOverflow)?
            || !value.fraction.is_finite()
            || !(0.0..=1.0).contains(&value.fraction)
            || !value.refinement_change.is_finite()
            || value.refinement_change < 0.0
        {
            return Err(DiagnosticExportError::InvalidReport);
        }
    }
    Ok(())
}

pub fn write<W: Write>(
    json: &mut Json<W>,
    sample: CoverageFamilySample,
) -> Result<(), DiagnosticExportError> {
    json.raw("{\"clock\":")?;
    v::clock(json, sample.clock())?;
    json.raw(",\"family_identity\":")?;
    json.hex(sample.family_identity())?;
    json.raw(",\"tracking_identity\":")?;
    json.hex(sample.tracking_identity())?;
    json.raw(",\"core\":")?;
    regions(json, sample.core())?;
    json.raw(",\"annulus\":")?;
    regions(json, sample.annulus())?;
    let metadata = sample.sampling();
    json.raw(",\"sampling\":{\"layout\":")?;
    v::layout(json, metadata.layout)?;
    json.raw(",\"points\":")?;
    json.counter(metadata.points)?;
    json.raw(",\"relative_floors\":")?;
    v::f64_array(json, metadata.relative_floors)?;
    json.raw(",\"sampled_core\":")?;
    counts(json, metadata.sampled_core)?;
    json.raw(",\"sampled_annulus\":")?;
    counts(json, metadata.sampled_annulus)?;
    json.raw("}}")
}

fn regions<W: Write>(
    json: &mut Json<W>,
    values: [RegionCoverage; 3],
) -> Result<(), DiagnosticExportError> {
    json.raw("[")?;
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            json.raw(",")?;
        }
        json.raw("{\"status\":")?;
        json.string(match value.status {
            CoverageStatus::Nonempty => "Nonempty",
            CoverageStatus::RegionEmpty => "RegionEmpty",
        })?;
        json.raw(",\"fraction\":")?;
        json.f64(value.fraction)?;
        json.raw(",\"refinement_change\":")?;
        json.f64(value.refinement_change)?;
        json.raw(",\"panels\":")?;
        json.counter(value.panels)?;
        json.raw(",\"evaluations\":")?;
        json.counter(value.evaluations)?;
        json.raw("}")?;
    }
    json.raw("]")
}

fn counts<W: Write>(
    json: &mut Json<W>,
    values: [Option<usize>; 24],
) -> Result<(), DiagnosticExportError> {
    json.raw("[")?;
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            json.raw(",")?;
        }
        if let Some(value) = value {
            json.counter(value)?;
        } else {
            json.raw("null")?;
        }
    }
    json.raw("]")
}
