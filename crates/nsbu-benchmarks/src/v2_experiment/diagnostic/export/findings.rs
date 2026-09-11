use super::{json::Json, values as v, DiagnosticExportError};
use crate::{
    regions::RegionalReport,
    v2_experiment::{
        binding::NodeBindingStatus,
        physical::{PhysicalExtrema, PhysicalRefinementSample, SampleMaximum},
        pressure::PressureRefinementSample,
        reference::regional::RegionalTrackingSample,
    },
};
use nsbu_solver::diagnostics::local::LocalError;
use std::io::Write;

fn local5<W: Write>(
    j: &mut Json<W>,
    space: [LocalError; 2],
    time: [LocalError; 2],
    method: LocalError,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"space\":[")?;
    v::local(j, space[0])?;
    j.raw(",")?;
    v::local(j, space[1])?;
    j.raw("],\"time\":[")?;
    v::local(j, time[0])?;
    j.raw(",")?;
    v::local(j, time[1])?;
    j.raw("],\"method\":")?;
    v::local(j, method)?;
    j.raw("}")
}
pub fn physical<W: Write>(
    j: &mut Json<W>,
    s: PhysicalRefinementSample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, s.clock())?;
    j.raw(",\"identity\":")?;
    j.hex(s.identity())?;
    j.raw(",\"sample_grid\":")?;
    v::layout(j, s.sample_layout())?;
    j.raw(",\"relative_floors\":")?;
    v::f64_array(j, s.relative_floors())?;
    j.raw(",\"quantities\":[")?;
    for (i, q) in s.quantities().iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        physical_quantity(j, *q)?
    }
    j.raw("]}")
}
fn physical_quantity<W: Write>(
    j: &mut Json<W>,
    q: crate::v2_experiment::physical::QuantityRefinement,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"quantity\":")?;
    j.string(v::quantity(q.quantity))?;
    j.raw(",\"comparisons\":")?;
    local5(j, q.space, q.time, q.method)?;
    j.raw(",\"extrema\":[")?;
    for k in 0..5 {
        if k > 0 {
            j.raw(",")?
        }
        extrema(
            j,
            q.extrema(k)
                .map_err(|_| DiagnosticExportError::InvalidReport)?,
        )?
    }
    j.raw("]}")
}
fn extrema<W: Write>(j: &mut Json<W>, e: PhysicalExtrema) -> Result<(), DiagnosticExportError> {
    j.raw("{\"error\":")?;
    maximum(j, e.error)?;
    j.raw(",\"relative_error\":")?;
    maximum(j, e.relative_error)?;
    j.raw(",\"reference\":")?;
    maximum(j, e.reference)?;
    j.raw("}")
}
fn maximum<W: Write>(j: &mut Json<W>, m: SampleMaximum) -> Result<(), DiagnosticExportError> {
    if let Some((l, linear, index, value)) = m.measured() {
        j.raw("{\"status\":\"Measured\",\"grid\":")?;
        v::layout(j, l)?;
        j.raw(",\"linear\":")?;
        j.usize(linear)?;
        j.raw(",\"index\":")?;
        v::usize_array(j, index)?;
        j.raw(",\"value\":")?;
        j.f64(value)?;
        j.raw("}")
    } else {
        j.raw("{\"status\":\"NoSamples\"}")
    }
}
pub fn pressure<W: Write>(
    j: &mut Json<W>,
    s: PressureRefinementSample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, s.clock())?;
    j.raw(",\"identity\":")?;
    j.hex(s.identity())?;
    j.raw(",\"source_domain\":")?;
    v::domain(j, s.source_domain())?;
    j.raw(",\"sample_grid\":")?;
    v::layout(j, s.sample_layout())?;
    j.raw(",\"force_grid\":")?;
    v::layout(j, s.force_layout())?;
    j.raw(",\"force_workers\":")?;
    j.counter(s.force_workers())?;
    j.raw(",\"relative_floors\":")?;
    v::f64_array(j, s.relative_floors())?;
    j.raw(",\"quantities\":[")?;
    for (i, q) in s.quantities().iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        pressure_quantity(j, *q)?
    }
    j.raw("]}")
}
fn pressure_quantity<W: Write>(
    j: &mut Json<W>,
    q: crate::v2_experiment::pressure::QuantityRefinement,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"quantity\":")?;
    j.string(v::quantity(q.quantity))?;
    j.raw(",\"comparisons\":")?;
    local5(j, q.space, q.time, q.method)?;
    j.raw("}")
}
pub fn regional<W: Write>(
    j: &mut Json<W>,
    s: RegionalTrackingSample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, s.clock())?;
    j.raw(",\"identity\":")?;
    j.hex(s.identity())?;
    j.raw(",\"sample_grid\":")?;
    v::layout(j, s.sample_layout())?;
    j.raw(",\"relative_floors\":")?;
    v::f64_array(j, s.relative_floors())?;
    j.raw(",\"branches\":[")?;
    for (i, b) in s.branches().iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        regional_branch(j, *b)?
    }
    j.raw("]}")
}
fn regional_branch<W: Write>(
    j: &mut Json<W>,
    b: crate::v2_experiment::reference::regional::RegionalBranchTracking,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"branch\":")?;
    j.usize(b.branch)?;
    j.raw(",\"quantities\":[")?;
    for (k, q) in b.quantities.into_iter().enumerate() {
        if k > 0 {
            j.raw(",")?
        }
        regional_quantity(j, q)?
    }
    j.raw("]}")
}
fn regional_quantity<W: Write>(
    j: &mut Json<W>,
    q: crate::v2_experiment::reference::regional::RegionalTrackingQuantity,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"quantity\":")?;
    j.string(v::quantity(q.quantity))?;
    j.raw(",\"global\":")?;
    v::local(j, q.global)?;
    j.raw(",\"regional\":")?;
    regional_report(j, q.regional)?;
    j.raw("}")
}
pub fn regional_report<W: Write>(
    j: &mut Json<W>,
    r: RegionalReport,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"components\":")?;
    j.usize(r.components)?;
    j.raw(",\"clock\":")?;
    v::clock(j, r.clock)?;
    j.raw(",\"dimensions\":")?;
    v::usize_array(j, r.dimensions)?;
    j.raw(",\"grid_complete\":")?;
    j.bool(r.grid_complete)?;
    j.raw(",\"global\":")?;
    v::sampled(j, r.global)?;
    regions(j, r.regions)?;
    j.raw(",\"root_work_charged\":")?;
    j.counter(r.root_work_charged)?;
    j.raw("}")
}
fn regions<W: Write>(
    j: &mut Json<W>,
    regions: [(
        crate::regions::SpatialRegion,
        nsbu_solver::diagnostics::local::SampledError,
    ); 5],
) -> Result<(), DiagnosticExportError> {
    j.raw(",\"regions\":[")?;
    for (i, (name, e)) in regions.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.raw("{\"region\":")?;
        j.string(v::region(name))?;
        j.raw(",\"finding\":")?;
        v::sampled(j, e)?;
        j.raw("}")?
    }
    j.raw("]")
}
pub fn binding<W: Write>(
    j: &mut Json<W>,
    s: crate::v2_experiment::binding::NodeBindingSample,
) -> Result<(), DiagnosticExportError> {
    j.raw("{\"clock\":")?;
    v::clock(j, s.clock())?;
    j.raw(",\"family_identity\":")?;
    j.hex(s.family_identity())?;
    j.raw(",\"probe_identity\":")?;
    j.hex(s.probe_identity())?;
    j.raw(",\"branches\":[")?;
    for (i, b) in s.branches().into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        binding_status(j, b)?
    }
    j.raw("]}")
}
fn binding_status<W: Write>(
    j: &mut Json<W>,
    b: NodeBindingStatus,
) -> Result<(), DiagnosticExportError> {
    match b {
        NodeBindingStatus::MissingRetainedNode => j.raw("{\"status\":\"MissingRetainedNode\"}"),
        NodeBindingStatus::Compared(p) => {
            j.raw("{\"status\":\"Compared\",\"provenance\":{\"clock\":")?;
            v::clock(j, p.clock)?;
            j.raw(",\"epoch\":")?;
            j.u128(p.epoch.0)?;
            j.raw(",\"accepted_steps\":")?;
            j.u128(p.accepted_steps)?;
            j.raw(",\"origin\":")?;
            j.string(v::origin(p.origin))?;
            j.raw(",\"coefficients_equal\":")?;
            j.bool(p.coefficients_equal)?;
            j.raw("}}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::diagnostics::local::SampledError;

    #[test]
    fn diagnostic_export_missing_variants_are_not_zero_measurements() {
        let mut bytes = Vec::new();
        let mut json = Json::new(&mut bytes);
        json.raw("[").unwrap();
        maximum(&mut json, SampleMaximum::NoSamples).unwrap();
        json.raw(",").unwrap();
        binding_status(&mut json, NodeBindingStatus::MissingRetainedNode).unwrap();
        json.raw(",").unwrap();
        v::sampled(&mut json, SampledError::NoSamples).unwrap();
        json.raw("]").unwrap();
        json.finish().unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded[0]["status"], "NoSamples");
        assert_eq!(decoded[1]["status"], "MissingRetainedNode");
        assert_eq!(decoded[2]["status"], "NoSamples");
    }
}
