use super::{json::Json, values as v, DiagnosticExportError, DiagnosticExportPlan};
use crate::v2_experiment::diagnostic::MissingChannel;
use std::io::Write;

pub fn write<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    j.raw("{")?;
    context_identity(j, p)?;
    context_numerical(j, p)?;
    context_schedule(j, p)?;
    context_diagnostics(j, p)?;
    context_reservations(j, p)?;
    j.raw(",\"missing_channels\":")?;
    missing_channels(j, &super::super::MISSING_CHANNELS)?;
    j.raw("}")
}
fn context_identity<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    j.raw("\"case\":\"similarity-mms-v2\",\"case_sha256\":")?;
    j.string(crate::CASE_SHA256)?;
    j.raw(",\"family_identity\":")?;
    j.hex(p.family_identity)?;
    j.raw(",\"probe_identity\":")?;
    j.hex(p.probe_identity)
}
fn context_numerical<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    let s = p.family;
    j.raw(",\"grids\":")?;
    v::usize_array(j, s.grids)?;
    j.raw(",\"steps\":[")?;
    for (i, x) in s.steps.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.u128(x)?
    }
    j.raw("]")?;
    branches(j, p.branches)?;
    j.raw(",\"force_grid\":")?;
    v::layout(j, s.force.samples)?;
    j.raw(",\"force_workers\":")?;
    j.counter(s.force.workers)?;
    j.raw(",\"endpoint_ticks\":")?;
    j.u128(s.endpoint)?;
    j.raw(",\"advective_limit\":")?;
    j.f64(s.advective_limit)?;
    tolerances(j, s.tolerances)
}
fn branches<W: Write>(
    j: &mut Json<W>,
    branches: [super::plan::BranchContext; 6],
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"branches\":[")?;
    for (i, b) in branches.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.raw("{\"grid\":")?;
        j.usize(b.grid)?;
        j.raw(",\"step_ticks\":")?;
        j.u128(b.step)?;
        j.raw(",\"method\":")?;
        j.string(match b.method {
            nsbu_solver::integrators::method::Method::CoxMatthews => "CoxMatthews",
            nsbu_solver::integrators::method::Method::HochbruckOstermann => "HochbruckOstermann",
        })?;
        j.raw("}")?
    }
    j.raw("]")
}
fn tolerances<W: Write>(
    j: &mut Json<W>,
    t: nsbu_solver::integrators::indicator::Tolerances,
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"tolerances\":{\"absolute\":")?;
    v::f64_array(j, t.absolute)?;
    j.raw(",\"relative\":")?;
    v::f64_array(j, t.relative)?;
    j.raw("}")
}
fn context_schedule<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    clocks(j, "accepted_clocks", p.accepted)?;
    clocks(j, "manifest_clocks", p.manifest)?;
    clocks(j, "residual_clocks", p.residual)
}
fn context_diagnostics<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    let d = p.settings;
    j.raw(",\"physical_samples\":")?;
    v::layout(j, d.physical_samples)?;
    j.raw(",\"pressure_samples\":")?;
    v::layout(j, d.pressure_samples)?;
    j.raw(",\"reference_samples\":")?;
    v::layout(j, d.reference_samples)?;
    j.raw(",\"physical_floors\":")?;
    v::f64_array(j, d.physical_floors)?;
    j.raw(",\"pressure_floors\":")?;
    v::f64_array(j, d.pressure_floors)?;
    j.raw(",\"reference_floors\":")?;
    v::f64_array(j, d.reference_floors)?;
    j.raw(",\"regional_root_budget\":")?;
    j.counter(d.regional_root_budget)
}
fn context_reservations<W: Write>(
    j: &mut Json<W>,
    p: DiagnosticExportPlan,
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"coordinator_reservation\":{\"storage_bytes\":")?;
    j.counter(p.coordinator.storage_bytes)?;
    j.raw(",\"joint_storage_bytes\":")?;
    j.counter(p.coordinator.joint_storage_bytes)?;
    j.raw(",\"event_attempts\":")?;
    j.counter(p.coordinator.work.attempts)?;
    j.raw(",\"accepted_events\":")?;
    j.counter(p.coordinator.work.accepted_events)?;
    j.raw(",\"residual_events\":")?;
    j.counter(p.coordinator.work.residual_events)?;
    j.raw("}")?;
    j.raw(",\"export_reservation\":{\"maximum_output_bytes\":")?;
    j.counter(p.bounds().maximum_output_bytes)?;
    j.raw(",\"maximum_byte_visits\":")?;
    j.counter(p.bounds().maximum_byte_visits)?;
    j.raw(",\"maximum_write_calls\":")?;
    j.counter(p.bounds().maximum_write_calls)?;
    j.raw("}")
}
pub fn missing_channels<W: Write>(
    j: &mut Json<W>,
    channels: &[MissingChannel],
) -> Result<(), DiagnosticExportError> {
    j.raw("[")?;
    for (i, m) in channels.iter().copied().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.string(missing(m))?
    }
    j.raw("]")
}
fn clocks<W: Write, const N: usize>(
    j: &mut Json<W>,
    name: &str,
    a: [nsbu_solver::domain::TickClock; N],
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"")?;
    j.raw(name)?;
    j.raw("\":[")?;
    for (i, c) in a.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        v::clock(j, c)?
    }
    j.raw("]")
}

fn missing(m: MissingChannel) -> &'static str {
    match m {
        MissingChannel::ForceResolution => "ForceResolution",
        MissingChannel::ForcePrecision => "ForcePrecision",
        MissingChannel::Arithmetic => "Arithmetic",
        MissingChannel::ReferencePrecision => "ReferencePrecision",
        MissingChannel::PressureReference => "PressureReference",
        MissingChannel::PressureGauge => "PressureGauge",
        MissingChannel::Transfer => "Transfer",
        MissingChannel::SamplingResolution => "SamplingResolution",
        MissingChannel::QuadratureResolution => "QuadratureResolution",
        MissingChannel::RegionVolumeCoverage => "RegionVolumeCoverage",
    }
}
