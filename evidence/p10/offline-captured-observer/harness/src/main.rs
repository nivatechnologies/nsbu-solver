//! Read-only offline conservative-balance observer for decoded captured snapshots.
//!
//! Experimental P10 diagnostic scope only: the output is a doubled-grid conservative
//! balance measurement (`scope=balance-diagnostic-only`, `qualification=false`). It is
//! not a fine-observable acceptance and qualifies no PDE window.
//! Reviewer note: `decode` and `model` are included verbatim from the reviewed
//! snapshot comparison adapter; this crate adds no copies and weakens no admission.
#[allow(dead_code)]
#[path = "../../../snapshot-comparison-adapter/harness/src/decode.rs"]
mod decode;
#[allow(dead_code)]
#[path = "../../../snapshot-comparison-adapter/harness/src/model.rs"]
mod model;
mod observer;

use model::ComparisonKind;
use nsbu_solver::{
    domain::TickClock,
    spectral::{FftBackend, FftCatalog},
};
use observer::{ObserverLedger, OfflineObserver};
use serde_json::{json, Value};
use std::path::Path;

const SCHEMA: &str = "p10-offline-captured-observer-v1";
const SCOPE: &str = "balance-diagnostic-only";
const USAGE: &str = "usage: p10-offline-captured-observer <preflight|run> <manifest.json> \
    <force-sample-dimension> <workers> <cap-bytes> <owned-radix|rustfft-6.4.1-avx-avx2-fma>\n\
    or:    p10-offline-captured-observer n512-ledger <clock-record.json> <cap-bytes>";

/// Live N512 observer profile from the copied clock record; arithmetic-only inputs
/// for the `n512-ledger` mode. Nothing here allocates state, observers or catalogs.
const LEDGER_DIMENSIONS: [usize; 3] = [512; 3];
const LEDGER_LENGTHS: [f64; 3] = [1.0; 3];
const LEDGER_VISCOSITY: f64 = 1.0;
const LEDGER_FORCE_SAMPLE: usize = 1024;
const LEDGER_WORKERS: usize = 32;
const LEDGER_BACKEND_NAME: &str = "rustfft-6.4.1-avx-avx2-fma";
const BACCUS_512_GIB_NOMINAL_BYTES: u128 = 549_755_813_888;

fn main() {
    match command(std::env::args().skip(1).collect()) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("offline-observer-refusal: {error}");
            std::process::exit(1);
        }
    }
}

fn command(mut arguments: Vec<String>) -> Result<String, String> {
    if arguments.first().is_some_and(|value| value == "--help") {
        return Ok(format!("{USAGE}\nschema={SCHEMA}\nscope={SCOPE}"));
    }
    let mode = match arguments.first().map(String::as_str) {
        Some("preflight") => Mode::Preflight,
        Some("run") => Mode::Run,
        Some("n512-ledger") => Mode::N512Ledger,
        _ => return Err(format!("unknown mode; {USAGE}")),
    };
    arguments.remove(0);
    if matches!(mode, Mode::N512Ledger) {
        let [record, cap] = arguments.as_slice() else {
            return Err(format!("n512-ledger expects two arguments; {USAGE}"));
        };
        return n512_ledger(Path::new(record), cap.parse().map_err(|_| "invalid cap")?);
    }
    let [manifest, sample, workers, cap, backend] = arguments.as_slice() else {
        return Err(format!("expected five arguments; {USAGE}"));
    };
    execute(
        mode,
        Path::new(manifest),
        sample.parse().map_err(|_| "invalid sample dimension")?,
        workers.parse().map_err(|_| "invalid worker count")?,
        cap.parse().map_err(|_| "invalid cap")?,
        backend,
    )
}

#[derive(Clone, Copy)]
enum Mode {
    Preflight,
    Run,
    N512Ledger,
}

/// Resource-only N512 ledger: pure arithmetic preflight for the live profile from the
/// copied clock record. It checks no execution availability and allocates no state,
/// observer, catalog, force table or snapshot; the nominal comparison is an estimate
/// and a fresh actual-memory measurement is still required before any N512 run.
fn n512_ledger(record_path: &Path, cap: usize) -> Result<String, String> {
    let record: Value = serde_json::from_slice(&std::fs::read(record_path).map_err(model::debug)?)
        .map_err(model::debug)?;
    let identity = record["identity"]
        .as_str()
        .ok_or("clock record carries no identity string")?;
    if record["resumable"].as_bool() != Some(false) {
        return Err("clock record does not declare a non-resumable captured state".into());
    }
    if record["qualification"].as_bool() != Some(false) {
        return Err("clock record does not declare qualification=false".into());
    }
    let domain =
        nsbu_solver::domain::Domain::new(LEDGER_DIMENSIONS, LEDGER_LENGTHS, LEDGER_VISCOSITY)
            .map_err(model::debug)?;
    let samples =
        nsbu_solver::domain::Layout::new([LEDGER_FORCE_SAMPLE; 3]).map_err(model::debug)?;
    let backend = parse_backend(LEDGER_BACKEND_NAME)?;
    let ledger =
        OfflineObserver::preflight(domain, samples, LEDGER_WORKERS, backend, identity.len())
            .map_err(model::debug)?;
    let total = u128::try_from(ledger.total_bytes).map_err(model::debug)?;
    let headroom = BACCUS_512_GIB_NOMINAL_BYTES.checked_sub(total);
    let mut value = json!({
        "schema": SCHEMA,
        "mode": "n512-ledger",
        "scope": SCOPE,
        "qualification": false,
        "basis": "arithmetic-preflight-only:no-availability-check:no-allocation-of-state-observer-catalog-force-table-snapshot",
        "clock_record": record_path.display().to_string(),
        "snapshot_identity_bytes": identity.len(),
        "profile": {
            "dimensions": LEDGER_DIMENSIONS,
            "lengths": LEDGER_LENGTHS,
            "viscosity": LEDGER_VISCOSITY,
            "force_sample_dimensions": [
                LEDGER_FORCE_SAMPLE,
                LEDGER_FORCE_SAMPLE,
                LEDGER_FORCE_SAMPLE
            ],
            "workers": LEDGER_WORKERS,
            "backend": LEDGER_BACKEND_NAME,
        },
        "budget": {
            "baccus_512_gib_nominal_bytes": BACCUS_512_GIB_NOMINAL_BYTES,
            "total_bytes": total,
            "headroom_vs_nominal_estimate_bytes": headroom,
            "fits_baccus_512_gib_nominal_estimate": headroom.is_some(),
            "nominal_note": "estimate only; a fresh actual-memory measurement of the target host is required before any N512 run",
        },
        "requested": { "cap_bytes": cap },
        "fits": ledger.total_bytes <= cap,
    });
    let object = value.as_object_mut().ok_or("internal ledger shape")?;
    object.insert("ledger".into(), ledger_value(&ledger));
    serde_json::to_string_pretty(&value).map_err(model::debug)
}

struct Prepared {
    domain: nsbu_solver::domain::Domain,
    samples: nsbu_solver::domain::Layout,
    workers: usize,
    cap: usize,
    backend: FftBackend,
    ledger: ObserverLedger,
}

fn execute(
    mode: Mode,
    manifest_path: &Path,
    sample: usize,
    workers: usize,
    cap: usize,
    backend_name: &str,
) -> Result<String, String> {
    let manifest = decode::read_manifest(manifest_path)?;
    admitted_profile(&manifest)?;
    let clock = clock_from(&manifest)?;
    let domain = manifest.domain()?;
    let samples = nsbu_solver::domain::Layout::new([sample; 3]).map_err(model::debug)?;
    let backend = parse_backend(backend_name)?;
    backend.ensure_available().map_err(model::debug)?;
    let ledger =
        OfflineObserver::preflight(domain, samples, workers, backend, manifest.identity.len())
            .map_err(model::debug)?;
    let binding = binding_value(
        &manifest,
        &clock,
        samples,
        workers,
        cap,
        backend_name,
        &ledger,
    );
    match mode {
        Mode::Preflight => {
            let mut value = binding;
            let object = value.as_object_mut().ok_or("internal binding shape")?;
            object.insert("fits".into(), json!(ledger.total_bytes <= cap));
            serde_json::to_string_pretty(&value).map_err(model::debug)
        }
        Mode::Run => run(
            manifest,
            clock,
            Prepared {
                domain,
                samples,
                workers,
                cap,
                backend,
                ledger,
            },
            binding,
        ),
        Mode::N512Ledger => {
            Err("internal: n512-ledger is dispatched before manifest admission".into())
        }
    }
}

fn run(
    manifest: model::Manifest,
    clock: TickClock,
    prepared: Prepared,
    mut binding: Value,
) -> Result<String, String> {
    let Prepared {
        domain,
        samples,
        workers,
        cap,
        backend,
        ledger,
    } = prepared;
    if ledger.total_bytes > cap {
        return Err(format!(
            "resource preflight refusal: admitted {} exceeds cap {cap}",
            ledger.total_bytes
        ));
    }
    let snapshot = decode::load(&manifest)?;
    let catalog = FftCatalog::new(backend, ledger.catalog_bytes).map_err(model::debug)?;
    let mut observer = OfflineObserver::new(
        domain,
        samples,
        workers,
        &catalog,
        manifest.identity.len(),
        &ledger,
        cap,
    )
    .map_err(model::debug)?;
    let balance = observer
        .observe(clock, snapshot.coefficients.each_ref().map(Vec::as_slice))
        .map_err(model::debug)?;
    if !balance_values_finite(&balance) {
        return Err("non-finite balance measurement".into());
    }
    let object = binding.as_object_mut().ok_or("internal binding shape")?;
    object.insert(
        "snapshot".into(),
        json!({
            "coefficient_sha256": snapshot.coefficient_sha256,
            "file_sha256": snapshot.file_sha256,
            "snapshot_elapsed": snapshot.clock.elapsed,
            "snapshot_target": snapshot.clock.target,
            "snapshot_epoch": snapshot.clock.epoch,
            "snapshot_accepted_steps": snapshot.clock.accepted_steps,
        }),
    );
    object.insert("balance".into(), balance_value(&balance));
    object.insert("finite".into(), json!(true));
    serde_json::to_string_pretty(&binding).map_err(model::debug)
}

fn admitted_profile(manifest: &model::Manifest) -> Result<(), String> {
    if manifest.evolution.method != "cox-matthews" {
        return Err("unsupported evolution method (only cox-matthews is admitted)".into());
    }
    if manifest.evolution.case_sha256 != nsbu_benchmarks::CASE_SHA256 {
        return Err(
            "unsupported case profile (only the frozen similarity-mms-v2 case is admitted)".into(),
        );
    }
    match manifest.comparison_kind {
        ComparisonKind::MatchedSpatial
        | ComparisonKind::TimeDiagnostic
        | ComparisonKind::MatchedM512SpatialDiagnostic
        | ComparisonKind::ForceResolutionDiagnostic
        | ComparisonKind::MixedForceSpaceDiagnostic => Ok(()),
        ComparisonKind::MethodDiagnostic => {
            Err("unsupported comparison profile (method-diagnostic)".into())
        }
    }
}

fn clock_from(manifest: &model::Manifest) -> Result<TickClock, String> {
    let target = manifest.evolution.clock_target;
    let elapsed = manifest.elapsed;
    let remaining = target
        .checked_sub(elapsed)
        .filter(|value| *value > 0)
        .ok_or("endpoint snapshots with zero remaining ticks are unsupported")?;
    TickClock::restore(
        manifest.evolution.quantum_exponent,
        target,
        elapsed,
        remaining,
    )
    .map_err(model::debug)
}

fn parse_backend(name: &str) -> Result<FftBackend, String> {
    match name {
        "owned-radix" => Ok(FftBackend::OwnedRadix),
        "rustfft-6.4.1-avx-avx2-fma" => Ok(FftBackend::RustFft6_4_1AvxFma),
        _ => Err(format!("unsupported FFT backend {name:?}")),
    }
}

fn binding_value(
    manifest: &model::Manifest,
    clock: &TickClock,
    samples: nsbu_solver::domain::Layout,
    workers: usize,
    cap: usize,
    backend_name: &str,
    ledger: &ObserverLedger,
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
        "comparison_kind": kind_name(manifest.comparison_kind),
        "manifest_backend": manifest.backend,
        "manifest_execution": manifest.execution,
        "evolution": manifest.evolution,
        "clock": {
            "exponent": clock.exponent(),
            "target": clock.target(),
            "elapsed": clock.elapsed(),
            "remaining": clock.remaining(),
        },
        "requested": {
            "force_sample_dimensions": samples.dimensions(),
            "workers": workers,
            "cap_bytes": cap,
            "backend": backend_name,
        },
        "ledger": ledger_value(ledger),
    })
}

fn ledger_value(ledger: &ObserverLedger) -> Value {
    json!({
        "basis": "conservative-admission-upper-bound-not-measured-allocator-peak",
        "exact_reservation_bytes": {
            "catalog": ledger.catalog_bytes,
            "force_storage": ledger.force_storage_bytes,
            "conservative_workspace": ledger.conservative_bytes,
        },
        "conservative_allowance_bytes": {
            "observer_arrays": ledger.observer_array_bytes,
            "snapshot_state": ledger.snapshot_state_bytes,
            "headers": ledger.header_allowance_bytes,
        },
        "catalog_bytes": ledger.catalog_bytes,
        "force_storage_bytes": ledger.force_storage_bytes,
        "conservative_bytes": ledger.conservative_bytes,
        "observer_array_bytes": ledger.observer_array_bytes,
        "snapshot_state_bytes": ledger.snapshot_state_bytes,
        "header_allowance_bytes": ledger.header_allowance_bytes,
        "total_bytes": ledger.total_bytes,
    })
}

fn kind_name(kind: ComparisonKind) -> &'static str {
    match kind {
        ComparisonKind::MatchedSpatial => "MATCHED_SPATIAL",
        ComparisonKind::MatchedM512SpatialDiagnostic => "MATCHED_M512_SPATIAL_DIAGNOSTIC",
        ComparisonKind::TimeDiagnostic => "TIME_DIAGNOSTIC",
        ComparisonKind::ForceResolutionDiagnostic => "FORCE_RESOLUTION_DIAGNOSTIC",
        ComparisonKind::MixedForceSpaceDiagnostic => "MIXED_FORCE_SPACE_DIAGNOSTIC",
        ComparisonKind::MethodDiagnostic => "METHOD_DIAGNOSTIC",
    }
}

fn balance_value(balance: &nsbu_solver::diagnostics::balances::BalanceSample) -> Value {
    json!({
        "l2": balance.norms.l2,
        "h1": balance.norms.h1,
        "vorticity_l2": balance.norms.vorticity_l2,
        "divergence_l2": balance.norms.divergence_l2,
        "energy": balance.energy,
        "enstrophy": balance.enstrophy,
        "energy_dissipation": balance.energy_dissipation,
        "forcing_work": balance.forcing_work,
        "stretching": balance.stretching,
        "enstrophy_dissipation": balance.enstrophy_dissipation,
        "vorticity_forcing": balance.vorticity_forcing,
    })
}

fn balance_values_finite(balance: &nsbu_solver::diagnostics::balances::BalanceSample) -> bool {
    [
        balance.norms.l2,
        balance.norms.h1,
        balance.norms.vorticity_l2,
        balance.norms.divergence_l2,
        balance.energy,
        balance.enstrophy,
        balance.energy_dissipation,
        balance.forcing_work,
        balance.stretching,
        balance.enstrophy_dissipation,
        balance.vorticity_forcing,
    ]
    .into_iter()
    .all(f64::is_finite)
}

#[cfg(test)]
mod control;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
