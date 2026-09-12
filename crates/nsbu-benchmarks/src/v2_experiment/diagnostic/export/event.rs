use super::{
    context, findings, json::Json, values as v, DiagnosticExportError, DiagnosticExportPlan,
};
use crate::v2_experiment::{
    diagnostic::{AcceptedSchedule, DiagnosticEvent, ResidualSchedule},
    probes::{residuals::ResidualFamilySample, ProbeSample},
    RefinementSample,
};
use std::io::Write;

pub fn document<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
    reports: &[DiagnosticEvent],
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"schema\":\"nsbu.diagnostic-event\",\"schema_version\":")?;
    j.usize(p.schema_version())?;
    j.raw(",\"scientific_status\":\"UnqualifiedDiagnostic\",\"context\":")?;
    context::write(j, p)?;
    j.raw(",\"events\":[")?;
    for (i, e) in reports.iter().copied().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        event(j, p, i, e)?
    }
    j.raw("]}")
}
fn event<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
    index: usize,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    validate_event(p, index, e)?;
    event_header(j, e)?;
    if p.schema_version() == 2 {
        j.raw(",\"reconstructed_physical\":")?;
        findings::probe_physical(j, e.reconstructed_physical())?;
    }
    event_accepted(j, e)?;
    event_residual(j, e)?;
    j.raw("}")
}
fn validate_event(
    p: DiagnosticExportPlan,
    index: usize,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    validate_top(p, index, e)?;
    if p.schema_version() == 2 {
        validate_probe_physical(p, e)?;
    }
    if let Some(a) = e.accepted().sample() {
        validate_accepted(p, e, a)?
    }
    if let Some(r) = e.residual().sample() {
        validate_residual(p, e, r)?
    }
    Ok(())
}
fn validate_probe_physical(
    p: DiagnosticExportPlan,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    let sample = e.reconstructed_physical();
    validate_probe_physical_identity(p, e, sample)?;
    validate_probe_physical_settings(p, sample)
}

fn validate_probe_physical_identity(
    p: DiagnosticExportPlan,
    e: DiagnosticEvent,
    sample: crate::v2_experiment::probes::physical::ProbePhysicalSample,
) -> Result<(), DiagnosticExportError> {
    if (sample.clock(), sample.identity()) != (e.clock(), p.probe_identity)
        || sample.reconstruction().clock() != e.probe().clock()
        || sample.reconstruction().identity() != e.probe().identity()
        || sample.origins() != e.probe().origins()
    {
        return Err(DiagnosticExportError::InvalidReport);
    }
    Ok(())
}

fn validate_probe_physical_settings(
    p: DiagnosticExportPlan,
    sample: crate::v2_experiment::probes::physical::ProbePhysicalSample,
) -> Result<(), DiagnosticExportError> {
    let quantities = crate::v2_experiment::probes::physical::PROBE_PHYSICAL_QUANTITIES;
    if sample.source_domains() != p.probe_domains
        || sample.sample_layout() != p.settings.physical_samples
        || sample.relative_floors().map(f64::to_bits)
            != p.settings.physical_floors.map(f64::to_bits)
        || sample.quantities().map(|item| item.quantity) != quantities
    {
        return Err(DiagnosticExportError::InvalidReport);
    }
    Ok(())
}
fn validate_top(
    p: DiagnosticExportPlan,
    index: usize,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    let actual = (
        e.clock(),
        e.family_identity(),
        e.probe_identity(),
        e.probe().clock(),
        e.probe().identity(),
        e.accepted().sample().is_some(),
        e.residual().sample().is_some(),
    );
    let expected = (
        p.manifest[index],
        p.family_identity,
        p.probe_identity,
        e.clock(),
        p.probe_identity,
        p.accepted.contains(&e.clock()),
        p.residual.contains(&e.clock()),
    );
    if actual != expected {
        return Err(DiagnosticExportError::InvalidReport);
    }
    Ok(())
}
fn validate_accepted(
    p: DiagnosticExportPlan,
    e: DiagnosticEvent,
    a: crate::v2_experiment::diagnostic::AcceptedDiagnostic,
) -> Result<(), DiagnosticExportError> {
    let clocks = [
        a.spectral.clock(),
        a.physical.clock(),
        a.pressure.clock(),
        a.regional_reference.clock(),
        a.node_binding.clock(),
    ];
    let identities = [
        a.spectral.identity(),
        a.physical.identity(),
        a.pressure.identity(),
        a.regional_reference.identity(),
        a.node_binding.family_identity(),
    ];
    if (clocks, identities, a.node_binding.probe_identity())
        != ([e.clock(); 5], [p.family_identity; 5], p.probe_identity)
    {
        return Err(DiagnosticExportError::InvalidReport);
    }
    validate_diagnostic_settings(p, e, a)
}
fn validate_diagnostic_settings(
    p: DiagnosticExportPlan,
    e: DiagnosticEvent,
    a: crate::v2_experiment::diagnostic::AcceptedDiagnostic,
) -> Result<(), DiagnosticExportError> {
    let settings = p.settings;
    let actual = (
        a.physical.sample_layout(),
        a.physical.relative_floors(),
        a.pressure.sample_layout(),
        a.pressure.relative_floors(),
        a.pressure.source_domain(),
        a.pressure.force_layout(),
        a.pressure.force_workers(),
        a.regional_reference.sample_layout(),
        a.regional_reference.relative_floors(),
    );
    let expected = (
        settings.physical_samples,
        settings.physical_floors,
        settings.pressure_samples,
        settings.pressure_floors,
        p.pressure_source,
        p.pressure_layout,
        p.family.force.workers,
        settings.reference_samples,
        settings.reference_floors,
    );
    let invalid_region = a
        .regional_reference
        .branches()
        .iter()
        .flat_map(|b| b.quantities)
        .any(|q| {
            q.regional.clock != e.clock()
                || q.regional.dimensions != settings.reference_samples.dimensions()
                || Some(q.regional.root_work_charged)
                    > settings
                        .reference_samples
                        .real_len()
                        .checked_mul(settings.regional_root_budget)
        });
    if actual != expected || invalid_region {
        return Err(DiagnosticExportError::InvalidReport);
    }
    Ok(())
}
fn validate_residual(
    p: DiagnosticExportPlan,
    e: DiagnosticEvent,
    r: ResidualFamilySample,
) -> Result<(), DiagnosticExportError> {
    let invalid_branch = r.branches().iter().any(|branch| {
        (branch.force_workers(), branch.force_sample_layout())
            != (p.family.force.workers, p.residual_force)
    });
    if (r.clock(), r.reconstruction().identity(), invalid_branch)
        != (e.clock(), p.probe_identity, false)
    {
        return Err(DiagnosticExportError::InvalidReport);
    }
    Ok(())
}
fn event_header<W: Write>(
    j: &mut Json<W>,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, e.clock())?;
    j.raw(",\"scientific_status\":\"UnqualifiedDiagnostic\",\"family_identity\":")?;
    j.hex(e.family_identity())?;
    j.raw(",\"probe_identity\":")?;
    j.hex(e.probe_identity())?;
    j.raw(",\"missing_channels\":")?;
    context::missing_channels(j, e.missing_channels())?;
    j.raw(",\"probe\":")?;
    probe(j, e.probe())
}
fn event_accepted<W: Write>(
    j: &mut Json<W>,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"accepted\":")?;
    match (e.accepted().schedule(), e.accepted().sample()) {
        (AcceptedSchedule::NotScheduledAtResidualClock, None) => {
            j.raw("{\"schedule\":\"NotScheduledAtResidualClock\",\"sample\":null}")?
        }
        (AcceptedSchedule::Measured, Some(a)) => {
            j.raw("{\"schedule\":\"Measured\",\"sample\":{")?;
            j.raw("\"spectral\":")?;
            spectral(j, a.spectral)?;
            j.raw(",\"physical\":")?;
            findings::physical(j, a.physical)?;
            j.raw(",\"pressure\":")?;
            findings::pressure(j, a.pressure)?;
            j.raw(",\"regional_reference\":")?;
            findings::regional(j, a.regional_reference)?;
            j.raw(",\"node_binding\":")?;
            findings::binding(j, a.node_binding)?;
            j.raw("}}")?
        }
        _ => return Err(DiagnosticExportError::InvalidReport),
    }
    Ok(())
}
fn event_residual<W: Write>(
    j: &mut Json<W>,
    e: DiagnosticEvent,
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"residual\":")?;
    match (e.residual().schedule(), e.residual().sample()) {
        (ResidualSchedule::NotScheduledAtAcceptedClock, None) => {
            j.raw("{\"schedule\":\"NotScheduledAtAcceptedClock\",\"sample\":null}")?
        }
        (ResidualSchedule::Measured, Some(r)) => {
            j.raw("{\"schedule\":\"Measured\",\"sample\":")?;
            residual(j, r)?;
            j.raw("}")?
        }
        _ => return Err(DiagnosticExportError::InvalidReport),
    }
    Ok(())
}
fn probe<W: Write>(j: &mut Json<W>, p: ProbeSample) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, p.clock())?;
    j.raw(",\"identity\":")?;
    j.hex(p.identity())?;
    j.raw(",\"origins\":[")?;
    for (i, o) in p.origins().iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.raw("{\"accepted_nodes\":[")?;
        for (k, c) in o.accepted_nodes.into_iter().enumerate() {
            if k > 0 {
                j.raw(",")?
            }
            v::clock(j, c)?
        }
        j.raw("],\"state_clock\":")?;
        v::clock(j, o.state_clock)?;
        j.raw("}")?
    }
    j.raw("],\"values\":")?;
    bands(j, p.values())?;
    j.raw(",\"derivatives\":")?;
    bands(j, p.derivatives())?;
    j.raw("}")
}
fn bands<W: Write>(
    j: &mut Json<W>,
    a: &[nsbu_solver::diagnostics::comparison::BandComparison; 5],
) -> Result<(), DiagnosticExportError> {
    j.raw("[")?;
    for (i, x) in a.iter().copied().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        v::band(j, x)?
    }
    j.raw("]")
}
fn spectral<W: Write>(j: &mut Json<W>, s: RefinementSample) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, s.clock())?;
    j.raw(",\"identity\":")?;
    j.hex(s.identity())?;
    j.raw(",\"space\":")?;
    bands2(j, s.space())?;
    j.raw(",\"time\":")?;
    bands2(j, s.time())?;
    j.raw(",\"method\":")?;
    v::band(j, s.method())?;
    j.raw("}")
}
fn bands2<W: Write>(
    j: &mut Json<W>,
    a: [nsbu_solver::diagnostics::comparison::BandComparison; 2],
) -> Result<(), DiagnosticExportError> {
    j.raw("[")?;
    v::band(j, a[0])?;
    j.raw(",")?;
    v::band(j, a[1])?;
    j.raw("]")
}
fn residual<W: Write>(
    j: &mut Json<W>,
    r: ResidualFamilySample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, r.clock())?;
    j.raw(",\"reconstruction\":")?;
    probe(j, r.reconstruction())?;
    j.raw(",\"branches\":[")?;
    for (i, b) in r.branches().iter().copied().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        residual_branch(j, b)?
    }
    j.raw("],\"comparisons\":")?;
    bands(j, r.comparisons())?;
    j.raw(",\"temporal_geometry\":[")?;
    for (i, g) in r.temporal_geometry().levels().into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        v::geometry(j, g)?
    }
    j.raw("]}")
}
fn residual_branch<W: Write>(
    j: &mut Json<W>,
    b: crate::v2_experiment::probes::residuals::ResidualSample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"norms\":")?;
    v::norms(j, b.norms())?;
    j.raw(",\"geometry\":")?;
    v::geometry(j, b.geometry())?;
    j.raw(",\"origin\":")?;
    j.string(v::origin(b.origin()))?;
    j.raw(",\"source_domain\":")?;
    v::domain(j, b.domain())?;
    j.raw(",\"diagnostic_domain\":")?;
    v::domain(j, b.diagnostic_domain())?;
    j.raw(",\"force_grid\":")?;
    v::layout(j, b.force_sample_layout())?;
    j.raw(",\"force_workers\":")?;
    j.counter(b.force_workers())?;
    j.raw("}")
}
