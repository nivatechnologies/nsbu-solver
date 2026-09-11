use super::{json::Json, DiagnosticExportError};
use crate::{regions::SpatialRegion, v2_run::Origin};
use nsbu_solver::{
    diagnostics::{
        comparison::BandComparison,
        local::{LocalError, SampledError},
        norms::Norms,
        physical::PhysicalQuantity,
    },
    domain::{Domain, Layout, TickClock},
    verification::reconstruction::OffStageProbe,
};
use std::io::Write;

pub fn clock<W: Write>(j: &mut Json<W>, c: TickClock) -> Result<(), DiagnosticExportError> {
    j.raw("{\"exponent\":")?;
    j.i32(c.exponent())?;
    j.raw(",\"target\":")?;
    j.u128(c.target())?;
    j.raw(",\"elapsed\":")?;
    j.u128(c.elapsed())?;
    j.raw(",\"remaining\":")?;
    j.u128(c.remaining())?;
    j.raw("}")
}
pub fn layout<W: Write>(j: &mut Json<W>, l: Layout) -> Result<(), DiagnosticExportError> {
    usize_array(j, l.dimensions())
}
pub fn domain<W: Write>(j: &mut Json<W>, d: Domain) -> Result<(), DiagnosticExportError> {
    j.raw("{\"grid\":")?;
    layout(j, d.layout())?;
    j.raw(",\"lengths\":")?;
    f64_array(j, d.lengths())?;
    j.raw(",\"viscosity\":")?;
    j.f64(d.viscosity())?;
    j.raw("}")
}
pub fn norms<W: Write>(j: &mut Json<W>, n: Norms) -> Result<(), DiagnosticExportError> {
    j.raw("{\"l2\":")?;
    j.f64(n.l2)?;
    j.raw(",\"h1\":")?;
    j.f64(n.h1)?;
    j.raw(",\"vorticity_l2\":")?;
    j.f64(n.vorticity_l2)?;
    j.raw(",\"divergence_l2\":")?;
    j.f64(n.divergence_l2)?;
    j.raw("}")
}
pub fn band<W: Write>(j: &mut Json<W>, b: BandComparison) -> Result<(), DiagnosticExportError> {
    j.raw("{\"full\":")?;
    norms(j, b.full)?;
    j.raw(",\"common\":")?;
    norms(j, b.common)?;
    j.raw(",\"newly_resolved\":")?;
    norms(j, b.newly_resolved)?;
    j.raw(",\"mean_error\":")?;
    f64_array(j, b.mean_error)?;
    j.raw("}")
}
pub fn local<W: Write>(j: &mut Json<W>, e: LocalError) -> Result<(), DiagnosticExportError> {
    j.raw("{\"components\":")?;
    j.usize(e.components)?;
    j.raw(",\"samples\":")?;
    j.counter(e.samples)?;
    j.raw(",\"rms_error\":")?;
    j.f64(e.rms_error)?;
    j.raw(",\"peak_error\":")?;
    j.f64(e.peak_error)?;
    j.raw(",\"peak_relative_error\":")?;
    j.f64(e.peak_relative_error)?;
    j.raw(",\"reference_peak\":")?;
    j.f64(e.reference_peak)?;
    j.raw(",\"relative_floor\":")?;
    j.f64(e.relative_floor)?;
    j.raw("}")
}
pub fn sampled<W: Write>(j: &mut Json<W>, e: SampledError) -> Result<(), DiagnosticExportError> {
    match e {
        SampledError::NoSamples => j.raw("{\"status\":\"NoSamples\"}"),
        SampledError::Measured(v) => {
            j.raw("{\"status\":\"Measured\",\"error\":")?;
            local(j, v)?;
            j.raw("}")
        }
    }
}
pub fn quantity(q: PhysicalQuantity) -> &'static str {
    match q {
        PhysicalQuantity::Scalar => "pressure",
        PhysicalQuantity::ScalarGradient => "pressure_gradient",
        PhysicalQuantity::Vector => "velocity",
        PhysicalQuantity::Gradient => "gradient",
        PhysicalQuantity::Hessian => "hessian",
        PhysicalQuantity::Vorticity => "vorticity",
    }
}
pub fn region(r: SpatialRegion) -> &'static str {
    match r {
        SpatialRegion::Core => "core",
        SpatialRegion::Annulus => "annulus",
        SpatialRegion::InteriorOutsideNominal => "interior_outside_nominal",
        SpatialRegion::Collar => "collar",
        SpatialRegion::Exterior => "exterior",
    }
}
pub fn origin(o: Origin) -> &'static str {
    match o {
        Origin::InternalFromRest => "InternalFromRest",
        Origin::ExternalUnverified => "ExternalUnverified",
    }
}
pub fn geometry<W: Write>(j: &mut Json<W>, g: OffStageProbe) -> Result<(), DiagnosticExportError> {
    j.raw("{\"nodes\":[")?;
    for (i, c) in g.nodes().into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        clock(j, c)?
    }
    j.raw("],\"probe\":")?;
    clock(j, g.time())?;
    j.raw("}")
}
pub fn f64_array<W: Write, const N: usize>(
    j: &mut Json<W>,
    a: [f64; N],
) -> Result<(), DiagnosticExportError> {
    j.raw("[")?;
    for (i, v) in a.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.f64(v)?
    }
    j.raw("]")
}
pub fn usize_array<W: Write, const N: usize>(
    j: &mut Json<W>,
    a: [usize; N],
) -> Result<(), DiagnosticExportError> {
    j.raw("[")?;
    for (i, v) in a.into_iter().enumerate() {
        if i > 0 {
            j.raw(",")?
        }
        j.usize(v)?
    }
    j.raw("]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_export_extreme_clock_fits_reserved_width() {
        let value = TickClock::restore(i32::MIN, u128::MAX, 0, u128::MAX).unwrap();
        let mut bytes = Vec::new();
        let mut json = Json::new(&mut bytes);
        clock(&mut json, value).unwrap();
        let (count, _, _) = json.finish().unwrap();
        assert_eq!(count, bytes.len());
        assert!(count <= super::super::plan::CLOCK_WIDTH);
        let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded["exponent"], i32::MIN);
        assert_eq!(decoded["target"], u128::MAX.to_string());
    }
}
