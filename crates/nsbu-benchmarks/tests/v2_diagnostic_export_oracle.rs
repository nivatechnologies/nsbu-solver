//! Independent decoded-value oracle for the full diagnostic event schema.
use nsbu_benchmarks::{
    regions::RegionalReport,
    v2_experiment::{
        binding::NodeBindingStatus,
        diagnostic::{AcceptedSchedule, DiagnosticEvent, ResidualSchedule},
        physical::{PhysicalExtrema, SampleMaximum},
        probes::ProbeSample,
    },
};
use nsbu_solver::{
    diagnostics::{
        comparison::BandComparison,
        local::{LocalError, SampledError},
        norms::Norms,
        physical::PhysicalQuantity,
    },
    domain::{Domain, Layout, TickClock},
};
use serde_json::Value;

/// Compare every decoded event field with its retained library report.
pub fn document(json: &Value, reports: &[DiagnosticEvent]) {
    assert_eq!(json["events"].as_array().unwrap().len(), reports.len());
    for (event, value) in reports
        .iter()
        .copied()
        .zip(json["events"].as_array().unwrap())
    {
        event_value(value, event)
    }
}
fn event_value(j: &Value, e: DiagnosticEvent) {
    clock(&j["clock"], e.clock());
    assert_eq!(j["family_identity"], hex(e.family_identity()));
    assert_eq!(j["probe_identity"], hex(e.probe_identity()));
    assert_eq!(j["scientific_status"], "UnqualifiedDiagnostic");
    assert_eq!(
        j["missing_channels"].as_array().unwrap().len(),
        e.missing_channels().len()
    );
    probe(&j["probe"], e.probe());
    match (e.accepted().schedule(), e.accepted().sample()) {
        (AcceptedSchedule::NotScheduledAtResidualClock, None) => {
            assert_eq!(j["accepted"]["schedule"], "NotScheduledAtResidualClock");
            assert!(j["accepted"]["sample"].is_null())
        }
        (AcceptedSchedule::Measured, Some(a)) => {
            assert_eq!(j["accepted"]["schedule"], "Measured");
            let x = &j["accepted"]["sample"];
            clock(&x["spectral"]["clock"], a.spectral.clock());
            assert_eq!(x["spectral"]["identity"], hex(a.spectral.identity()));
            let bands = [
                a.spectral.space()[0],
                a.spectral.space()[1],
                a.spectral.time()[0],
                a.spectral.time()[1],
                a.spectral.method(),
            ];
            for (i, b) in bands.into_iter().enumerate() {
                let key = if i < 2 {
                    "space"
                } else if i < 4 {
                    "time"
                } else {
                    "method"
                };
                let value = if i < 2 {
                    &x["spectral"][key][i]
                } else if i < 4 {
                    &x["spectral"][key][i - 2]
                } else {
                    &x["spectral"][key]
                };
                band(value, b)
            }
            physical(&x["physical"], a.physical);
            pressure(&x["pressure"], a.pressure);
            regional(&x["regional_reference"], a.regional_reference);
            binding(&x["node_binding"], a.node_binding)
        }
        _ => panic!("invalid accepted variant"),
    }
    match (e.residual().schedule(), e.residual().sample()) {
        (ResidualSchedule::NotScheduledAtAcceptedClock, None) => {
            assert_eq!(j["residual"]["schedule"], "NotScheduledAtAcceptedClock");
            assert!(j["residual"]["sample"].is_null())
        }
        (ResidualSchedule::Measured, Some(r)) => {
            assert_eq!(j["residual"]["schedule"], "Measured");
            let x = &j["residual"]["sample"];
            clock(&x["clock"], r.clock());
            probe(&x["reconstruction"], r.reconstruction());
            for (i, b) in r.branches().iter().copied().enumerate() {
                norms(&x["branches"][i]["norms"], b.norms());
                geometry(&x["branches"][i]["geometry"], b.geometry());
                assert_eq!(x["branches"][i]["origin"], format!("{:?}", b.origin()));
                domain(&x["branches"][i]["source_domain"], b.domain());
                domain(
                    &x["branches"][i]["diagnostic_domain"],
                    b.diagnostic_domain(),
                );
                layout(&x["branches"][i]["force_grid"], b.force_sample_layout());
                assert_eq!(
                    x["branches"][i]["force_workers"],
                    b.force_workers().to_string()
                )
            }
            for (i, b) in r.comparisons().iter().copied().enumerate() {
                band(&x["comparisons"][i], b)
            }
            for (i, g) in r.temporal_geometry().levels().into_iter().enumerate() {
                geometry(&x["temporal_geometry"][i], g)
            }
        }
        _ => panic!("invalid residual variant"),
    }
}
fn probe(j: &Value, p: ProbeSample) {
    clock(&j["clock"], p.clock());
    assert_eq!(j["identity"], hex(p.identity()));
    for (i, o) in p.origins().iter().enumerate() {
        for (k, c) in o.accepted_nodes.into_iter().enumerate() {
            clock(&j["origins"][i]["accepted_nodes"][k], c)
        }
        clock(&j["origins"][i]["state_clock"], o.state_clock)
    }
    for (i, b) in p.values().iter().copied().enumerate() {
        band(&j["values"][i], b)
    }
    for (i, b) in p.derivatives().iter().copied().enumerate() {
        band(&j["derivatives"][i], b)
    }
}
fn physical(j: &Value, s: nsbu_benchmarks::v2_experiment::physical::PhysicalRefinementSample) {
    clock(&j["clock"], s.clock());
    assert_eq!(j["identity"], hex(s.identity()));
    layout(&j["sample_grid"], s.sample_layout());
    words(&j["relative_floors"], &s.relative_floors());
    for (i, q) in s.quantities().iter().enumerate() {
        assert_eq!(j["quantities"][i]["quantity"], quantity(q.quantity));
        let x = &j["quantities"][i]["comparisons"];
        local5(x, q.space, q.time, q.method);
        for k in 0..5 {
            extrema(&j["quantities"][i]["extrema"][k], q.extrema(k).unwrap())
        }
    }
}
fn pressure(j: &Value, s: nsbu_benchmarks::v2_experiment::pressure::PressureRefinementSample) {
    clock(&j["clock"], s.clock());
    assert_eq!(j["identity"], hex(s.identity()));
    domain(&j["source_domain"], s.source_domain());
    layout(&j["sample_grid"], s.sample_layout());
    layout(&j["force_grid"], s.force_layout());
    words(&j["relative_floors"], &s.relative_floors());
    assert_eq!(j["force_workers"], s.force_workers().to_string());
    for (i, q) in s.quantities().iter().enumerate() {
        assert_eq!(j["quantities"][i]["quantity"], quantity(q.quantity));
        local5(
            &j["quantities"][i]["comparisons"],
            q.space,
            q.time,
            q.method,
        )
    }
}
fn local5(j: &Value, space: [LocalError; 2], time: [LocalError; 2], method: LocalError) {
    for i in 0..2 {
        local(&j["space"][i], space[i]);
        local(&j["time"][i], time[i])
    }
    local(&j["method"], method)
}
fn regional(
    j: &Value,
    s: nsbu_benchmarks::v2_experiment::reference::regional::RegionalTrackingSample,
) {
    clock(&j["clock"], s.clock());
    assert_eq!(j["identity"], hex(s.identity()));
    layout(&j["sample_grid"], s.sample_layout());
    words(&j["relative_floors"], &s.relative_floors());
    for (i, b) in s.branches().iter().enumerate() {
        assert_eq!(j["branches"][i]["branch"], b.branch);
        for (k, q) in b.quantities.iter().enumerate() {
            assert_eq!(
                j["branches"][i]["quantities"][k]["quantity"],
                quantity(q.quantity)
            );
            local(&j["branches"][i]["quantities"][k]["global"], q.global);
            regional_report(&j["branches"][i]["quantities"][k]["regional"], q.regional)
        }
    }
}
fn regional_report(j: &Value, r: RegionalReport) {
    clock(&j["clock"], r.clock);
    assert_eq!(j["components"], r.components);
    assert_eq!(j["grid_complete"], r.grid_complete);
    assert_eq!(j["root_work_charged"], r.root_work_charged.to_string());
    sampled(&j["global"], r.global);
    for (i, (region, e)) in r.regions.into_iter().enumerate() {
        assert_eq!(
            j["regions"][i]["region"],
            format!("{:?}", region)
                .to_ascii_lowercase()
                .replace("interioroutsidenominal", "interior_outside_nominal")
        );
        sampled(&j["regions"][i]["finding"], e)
    }
}
fn binding(j: &Value, s: nsbu_benchmarks::v2_experiment::binding::NodeBindingSample) {
    clock(&j["clock"], s.clock());
    assert_eq!(j["family_identity"], hex(s.family_identity()));
    assert_eq!(j["probe_identity"], hex(s.probe_identity()));
    for (i, b) in s.branches().into_iter().enumerate() {
        match b {
            NodeBindingStatus::MissingRetainedNode => {
                assert_eq!(j["branches"][i]["status"], "MissingRetainedNode")
            }
            NodeBindingStatus::Compared(p) => {
                let x = &j["branches"][i];
                assert_eq!(x["status"], "Compared");
                clock(&x["provenance"]["clock"], p.clock);
                assert_eq!(x["provenance"]["epoch"], p.epoch.0.to_string());
                assert_eq!(
                    x["provenance"]["accepted_steps"],
                    p.accepted_steps.to_string()
                );
                assert_eq!(x["provenance"]["origin"], format!("{:?}", p.origin));
                assert_eq!(x["provenance"]["coefficients_equal"], p.coefficients_equal)
            }
        }
    }
}
fn extrema(j: &Value, e: PhysicalExtrema) {
    maximum(&j["error"], e.error);
    maximum(&j["relative_error"], e.relative_error);
    maximum(&j["reference"], e.reference)
}
fn maximum(j: &Value, m: SampleMaximum) {
    match m.measured() {
        None => assert_eq!(j["status"], "NoSamples"),
        Some((layout_value, linear, index, value)) => {
            assert_eq!(j["status"], "Measured");
            assert_eq!(j["linear"], linear);
            layout(&j["grid"], layout_value);
            for (i, v) in index.into_iter().enumerate() {
                assert_eq!(j["index"][i], v)
            }
            word(&j["value"], value)
        }
    }
}
fn sampled(j: &Value, e: SampledError) {
    match e {
        SampledError::NoSamples => assert_eq!(j["status"], "NoSamples"),
        SampledError::Measured(e) => {
            assert_eq!(j["status"], "Measured");
            local(&j["error"], e)
        }
    }
}
fn local(j: &Value, e: LocalError) {
    assert_eq!(j["components"], e.components);
    assert_eq!(j["samples"], e.samples.to_string());
    word(&j["rms_error"], e.rms_error);
    word(&j["peak_error"], e.peak_error);
    word(&j["peak_relative_error"], e.peak_relative_error);
    word(&j["reference_peak"], e.reference_peak);
    word(&j["relative_floor"], e.relative_floor)
}
fn band(j: &Value, b: BandComparison) {
    norms(&j["full"], b.full);
    norms(&j["common"], b.common);
    norms(&j["newly_resolved"], b.newly_resolved);
    for (i, v) in b.mean_error.into_iter().enumerate() {
        word(&j["mean_error"][i], v)
    }
}
fn norms(j: &Value, n: Norms) {
    word(&j["l2"], n.l2);
    word(&j["h1"], n.h1);
    word(&j["vorticity_l2"], n.vorticity_l2);
    word(&j["divergence_l2"], n.divergence_l2)
}
fn geometry(j: &Value, g: nsbu_solver::verification::reconstruction::OffStageProbe) {
    for (i, c) in g.nodes().into_iter().enumerate() {
        clock(&j["nodes"][i], c)
    }
    clock(&j["probe"], g.time())
}
fn clock(j: &Value, c: TickClock) {
    assert_eq!(j["exponent"], c.exponent());
    assert_eq!(j["target"], c.target().to_string());
    assert_eq!(j["elapsed"], c.elapsed().to_string());
    assert_eq!(j["remaining"], c.remaining().to_string())
}
fn layout(j: &Value, l: Layout) {
    for (i, value) in l.dimensions().into_iter().enumerate() {
        assert_eq!(j[i], value)
    }
}
fn domain(j: &Value, d: Domain) {
    layout(&j["grid"], d.layout());
    words(&j["lengths"], &d.lengths());
    word(&j["viscosity"], d.viscosity())
}
fn words(j: &Value, values: &[f64]) {
    for (i, value) in values.iter().copied().enumerate() {
        word(&j[i], value)
    }
}
fn quantity(q: PhysicalQuantity) -> &'static str {
    match q {
        PhysicalQuantity::Scalar => "pressure",
        PhysicalQuantity::ScalarGradient => "pressure_gradient",
        PhysicalQuantity::Vector => "velocity",
        PhysicalQuantity::Gradient => "gradient",
        PhysicalQuantity::Hessian => "hessian",
        PhysicalQuantity::Vorticity => "vorticity",
    }
}
fn word(j: &Value, value: f64) {
    assert_eq!(j.as_f64().unwrap().to_bits(), value.to_bits())
}
fn hex(value: [u8; 32]) -> String {
    value.into_iter().map(|b| format!("{b:02x}")).collect()
}
