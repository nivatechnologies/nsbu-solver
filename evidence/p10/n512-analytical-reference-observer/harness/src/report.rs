//! JSON evidence assembly. Every reported value is echoed with its provenance;
//! outputs always carry `qualification=false` and zero accepted windows.
use nsbu_solver::diagnostics::local::SampledError;
use serde_json::{json, Value};

use crate::{
    ledger::{ObserverLedger, WorkLedger},
    model::{self, Manifest},
    observer::Observations,
};

pub const SCHEMA: &str = "p10-n512-analytical-reference-observer-v1";
pub const SCOPE: &str = "analytical-reference-observer:unqualified-sampled-diagnostic";

pub(crate) fn ledger_value(ledger: &ObserverLedger) -> Value {
    json!({
        "basis": "conservative-admission-upper-bound-not-measured-allocator-peak",
        "exact_reservation_bytes": {
            "catalog": ledger.catalog_bytes,
            "force_storage": ledger.force_storage_bytes,
            "conservative_workspace": ledger.conservative_bytes,
            "velocity_sampler": ledger.velocity_sampler_bytes,
            "pressure_sampler": ledger.pressure_sampler_bytes,
            "reference_cache": ledger.reference_cache_bytes,
        },
        "live_array_bytes": {
            "basis": "derived exactly from ReferenceObserver field allocations",
            "observer_arrays": ledger.observer_array_bytes,
            "comparison_arrays": ledger.comparison_array_bytes,
        },
        "conservative_allowance_bytes": {
            "snapshot_state": ledger.snapshot_state_bytes,
            "headers": ledger.header_allowance_bytes,
        },
        "total_bytes": ledger.total_bytes,
    })
}

pub(crate) fn work_value(work: &WorkLedger, root_budget: usize) -> Value {
    json!({
        "velocity_points": work.velocity_points,
        "pressure_points": work.pressure_points,
        "reference_evaluations": work.reference_evaluations,
        "reference_root_iterations_allowance": work.reference_root_iterations,
        "classification_root_budget": work.classification_root_budget,
        "root_budget_per_classification": root_budget,
        "observer_scalar_transforms": work.observer_scalar_transforms,
        "conservative_scalar_transforms": work.conservative_scalar_transforms,
        "provider_scalar_transforms_allowance": work.provider_scalar_transforms,
        "provider_work_units_allowance": work.provider_work_units,
        "weighted_visits": work.weighted_visits,
    })
}

pub(crate) fn binding_value(
    manifest: &Manifest,
    ledger: &ObserverLedger,
    work: &WorkLedger,
    request: &Value,
    rounding_estimates: [f64; 2],
) -> Value {
    json!({
        "schema": SCHEMA,
        "scope": SCOPE,
        "qualification": false,
        "accepted_windows": 0,
        "manifest_identity": manifest.identity,
        "source_commit": manifest.source_commit,
        "plan_sha256": manifest.plan_sha256,
        "reviewed_file_sha256": manifest.file_sha256,
        "reviewed_coefficient_sha256": manifest.coefficient_sha256,
        "manifest_backend": manifest.backend,
        "manifest_execution": manifest.execution,
        "evolution": manifest.evolution,
        "identity_binding": {
            "case_sha256": manifest.evolution.case_sha256,
            "reviewed_case_sha256": nsbu_benchmarks::CASE_SHA256,
            "case_field": identity_field(&manifest.identity, "case"),
            "retained_field": identity_field(&manifest.identity, "retained"),
            "provider_field": identity_field(&manifest.identity, "provider"),
        },
        "clock": {
            "exponent": manifest.evolution.quantum_exponent,
            "target": manifest.evolution.clock_target,
            "elapsed": manifest.elapsed,
            "remaining": manifest.evolution.clock_target - manifest.elapsed,
            "epoch": manifest.epoch,
            "accepted_steps": manifest.accepted_steps,
            "physical_rounding_estimates": rounding_estimates,
            "rounding_note": "separate elapsed/remaining dyadic conversion estimates from BenchmarkTime; exact significands receive zero",
            "synchronization": "one exact TickClock drives the captured state, the prescribed force, the analytical reference and every region classification",
        },
        "requested": request,
        "ledger": ledger_value(ledger),
        "work": work_value(work, request["root_budget"].as_u64().unwrap_or(0) as usize),
    })
}

pub(crate) fn observation_value(
    observations: &Observations,
    coefficient_sha256: &str,
    clock: &Value,
) -> Value {
    let quantities = observations
        .quantities
        .iter()
        .map(|quantity| {
            json!({
                "name": quantity.name,
                "kind": quantity.quantity,
                "components": quantity.components,
                "actual_scalar_transforms": quantity.transforms,
                "sample_layout": quantity.layout.dimensions(),
                "global": {
                    "samples": quantity.global.samples,
                    "rms_error": quantity.global.rms_error,
                    "peak_error": quantity.global.peak_error,
                    "peak_relative_error": quantity.global.peak_relative_error,
                    "reference_peak": quantity.global.reference_peak,
                    "relative_floor": quantity.global.relative_floor,
                },
                "regions": quantity.regions.iter().map(|region| json!({
                    "region": region.region,
                    "error": sampled_error(&region.error),
                })).collect::<Vec<_>>(),
                "grid_complete": quantity.grid_complete,
                "classification_root_work_charged": quantity.root_work_charged,
                "peaks": {
                    "error": peak(&quantity.error_peak),
                    "relative_error": peak(&quantity.relative_peak),
                    "reference": peak(&quantity.reference_peak),
                    "actual": peak(&quantity.actual_peak),
                    "actual_vs_reference_peak_height_error": quantity.peak_height_error,
                    "peak_location_periodic_distance_cells": quantity.peak_distance,
                    "distance_note": "per-axis minimum-image distance on the periodic sample lattice between the actual and reference peak locations",
                },
            })
        })
        .collect::<Vec<_>>();
    json!({
        "snapshot": {
            "coefficient_sha256": coefficient_sha256,
            "clock": clock,
            "state_read_only": "coefficients were borrowed and re-hashed unchanged after observation",
        },
        "quantities": quantities,
        "pressure_gauge": {
            "kind": "measured-analytical-sample-lattice-mean-subtracted",
            "measured_analytical_lattice_mean": observations.pressure_gauge_mean,
            "actual_pressure_gauge": "mean-zero by the reviewed conservative construction",
            "actual_pressure_lattice_mean_witness": observations.actual_pressure_lattice_mean,
            "imported_high_precision_gauge_artifact": "not-imported;separate-required-comparison",
            "note": "the analytical lattice mean is an empirical floating mean over the declared sample lattice; local pressure means are never removed independently",
        },
        "precision_identity": {
            "reference_evaluator": "nsbu_benchmarks::fields::reference (independent Rust jet evaluator, binary64)",
            "force_provider": "nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force (prescribed exact-v2 force)",
            "case_sha256": nsbu_benchmarks::CASE_SHA256,
            "provider_last_root_iterations": observations.provider_root_iterations,
            "executed_scalar_transforms": observations.executed_scalar_transforms,
        },
        "reference_precision_refinement": "required-separate-comparison:not-performed-here",
        "finite": true,
    })
}

fn peak(peak: &crate::quantity::FieldPeak) -> Value {
    json!({
        "linear": peak.maximum.linear,
        "index": peak.maximum.index,
        "value": peak.maximum.value,
        "source": peak.source.as_str(),
        "field_identity": peak.identity(),
        "order": "x-major z-fastest; first maximizer",
    })
}

fn sampled_error(error: &SampledError) -> Value {
    match error {
        SampledError::NoSamples => json!({"state": "no-samples"}),
        SampledError::Measured(value) => json!({
            "state": "measured",
            "samples": value.samples,
            "rms_error": value.rms_error,
            "peak_error": value.peak_error,
            "peak_relative_error": value.peak_relative_error,
            "reference_peak": value.reference_peak,
            "relative_floor": value.relative_floor,
        }),
    }
}

fn identity_field(identity: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    identity
        .split(';')
        .find_map(|field| field.strip_prefix(prefix.as_str()))
        .unwrap_or_default()
        .to_owned()
}

/// Refuse publication unless every reported measurement is finite.
pub(crate) fn all_finite(observations: &Observations) -> bool {
    let quantity_ok = |global: &nsbu_solver::diagnostics::local::LocalError| {
        [
            global.rms_error,
            global.peak_error,
            global.peak_relative_error,
            global.reference_peak,
            global.relative_floor,
        ]
        .into_iter()
        .all(f64::is_finite)
    };
    let error_ok = |error: &SampledError| match error {
        SampledError::NoSamples => true,
        SampledError::Measured(value) => [
            value.rms_error,
            value.peak_error,
            value.peak_relative_error,
            value.reference_peak,
            value.relative_floor,
        ]
        .into_iter()
        .all(f64::is_finite),
    };
    observations
        .quantities
        .iter()
        .all(|quantity| {
            quantity_ok(&quantity.global)
                && quantity.regions.iter().all(|region| error_ok(&region.error))
                && quantity.error_peak.maximum.value.is_finite()
                && quantity.relative_peak.maximum.value.is_finite()
                && quantity.reference_peak.maximum.value.is_finite()
                && quantity.actual_peak.maximum.value.is_finite()
                && quantity.peak_height_error.is_finite()
        })
        && observations.pressure_gauge_mean.is_finite()
        && observations.actual_pressure_lattice_mean.is_finite()
}

pub(crate) fn pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(model::debug)
}
