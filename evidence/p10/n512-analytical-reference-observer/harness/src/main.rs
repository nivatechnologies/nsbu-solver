//! Read-only offline N512 exact-v2 analytical reference observer for captured snapshots.
//!
//! Experimental P10 diagnostic scope only: the output compares an already decoded
//! immutable captured state against the independent analytical exact-v2 reference
//! at the snapshot's own physical clock (`scope`, `qualification=false`,
//! `accepted_windows=0` in every output). It never evolves, resets, replaces,
//! injects, resumes, recenters, aligns or phase-shifts an integrated state, and it
//! qualifies no PDE window. Reference-precision refinement (independent
//! high-precision arithmetic and imported empirical gauge artifacts) remains a
//! separate required comparison and is never claimed here.
#[allow(dead_code)]
#[path = "../../../snapshot-comparison-adapter/harness/src/decode.rs"]
mod decode;
#[allow(dead_code)]
#[path = "../../../snapshot-comparison-adapter/harness/src/model.rs"]
mod model;
mod cache;
mod ledger;
mod n512;
mod observer;
mod plan;
mod publication;
mod provenance;
mod record;
mod quantity;
mod report;
mod source_bind;

use model::ComparisonKind;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    spectral::{FftBackend, FftCatalog},
};
use report::{SCOPE, SCHEMA};
use serde_json::{json, Value};
use std::path::Path;

const USAGE: &str = "usage: p10-n512-analytical-reference-observer <preflight|run> \
    <manifest.json> <velocity-sample-dim> <pressure-sample-dim> <force-sample-dim> <workers> \
    <root-budget> <max-reference-evaluations> <cap-bytes> <velocity-floor> <pressure-floor> \
    <owned-radix|rustfft-6.4.1-avx-avx2-fma> [output.json]\n\
    or:    p10-n512-analytical-reference-observer n512-ledger <clock-record.json> \
    <velocity-sample-dim> <pressure-sample-dim> <force-sample-dim> <root-budget> \
    <max-reference-evaluations> <cap-bytes> [output.json]";

/// Live N512 profile constants for the arithmetic-only `n512-ledger` mode, taken
/// from the copied clock record. Nothing in this mode allocates state, observers,
/// catalogs, force tables, caches or snapshots.
pub(crate) const LEDGER_DIMENSIONS: [usize; 3] = [512; 3];
pub(crate) const LEDGER_LENGTHS: [f64; 3] = [1.0; 3];
pub(crate) const LEDGER_VISCOSITY: f64 = 1.0;
pub(crate) const LEDGER_WORKERS: usize = 32;
pub(crate) const LEDGER_BACKEND_NAME: &str = "rustfft-6.4.1-avx-avx2-fma";

fn main() {
    match command(std::env::args().skip(1).collect()) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("analytical-reference-observer-refusal: {error}");
            std::process::exit(1);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Preflight,
    Run,
    N512Ledger,
}

#[derive(Clone)]
struct Request {
    velocity_samples: usize,
    pressure_samples: usize,
    force_samples: usize,
    workers: usize,
    root_budget: usize,
    max_reference_evaluations: usize,
    cap: usize,
    velocity_floor: f64,
    pressure_floor: f64,
    backend: String,
}

impl Request {
    fn validate(&self) -> Result<(), String> {
        if self.workers == 0 || !(1..=128).contains(&self.root_budget) {
            return Err("invalid worker count or root budget".into());
        }
        if self.max_reference_evaluations == 0 || self.cap == 0 {
            return Err("missing work or memory cap".into());
        }
        if self.cap > 1 << 60 || self.max_reference_evaluations > 1 << 50 {
            return Err("cap or evaluation budget exceeds bounded admission".into());
        }
        for floor in [self.velocity_floor, self.pressure_floor] {
            if !floor.is_finite() || floor <= 0.0 {
                return Err("relative floors must be finite and positive".into());
            }
        }
        Ok(())
    }

    fn value(&self) -> Value {
        json!({
            "velocity_sample_dimension": self.velocity_samples,
            "pressure_sample_dimension": self.pressure_samples,
            "force_sample_dimension": self.force_samples,
            "workers": self.workers,
            "root_budget": self.root_budget,
            "max_reference_evaluations": self.max_reference_evaluations,
            "cap_bytes": self.cap,
            "velocity_relative_floor": self.velocity_floor,
            "pressure_relative_floor": self.pressure_floor,
            "backend": self.backend,
        })
    }
}

fn command(arguments: Vec<String>) -> Result<String, String> {
    let mut arguments = arguments.into_iter();
    let mode = match arguments.next().as_deref() {
        Some("preflight") => Mode::Preflight,
        Some("run") => Mode::Run,
        Some("n512-ledger") => Mode::N512Ledger,
        Some("--help") => return Ok(format!("{USAGE}\nschema={SCHEMA}\nscope={SCOPE}")),
        _ => return Err(format!("unknown mode; {USAGE}")),
    };
    let rest = arguments.collect::<Vec<_>>();
    if mode == Mode::N512Ledger {
        let (record, request, output) = ledger_arguments(&rest)?;
        return n512::n512_ledger(&record, request, output.as_deref());
    }
    let (manifest, request, output) = live_arguments(&rest)?;
    execute(mode, &manifest, request, output.as_deref())
}

/// Parse the seven-or-eight-argument `n512-ledger` invocation.
fn ledger_arguments(
    rest: &[String],
) -> Result<(std::path::PathBuf, Request, Option<std::path::PathBuf>), String> {
    let [record, velocity, pressure, force, budget, evaluations, cap, output @ ..] = rest else {
        return Err(format!("n512-ledger expects seven arguments; {USAGE}"));
    };
    let request = Request {
        velocity_samples: parse(velocity)?,
        pressure_samples: parse(pressure)?,
        force_samples: parse(force)?,
        workers: LEDGER_WORKERS,
        root_budget: parse(budget)?,
        max_reference_evaluations: parse(evaluations)?,
        cap: parse(cap)?,
        velocity_floor: 1.0,
        pressure_floor: 1.0,
        backend: LEDGER_BACKEND_NAME.to_owned(),
    };
    Ok((
        std::path::PathBuf::from(record),
        request,
        output.first().map(std::path::PathBuf::from),
    ))
}

/// Parse the eleven-or-twelve-argument `preflight`/`run` invocation.
fn live_arguments(
    rest: &[String],
) -> Result<(std::path::PathBuf, Request, Option<std::path::PathBuf>), String> {
    let [manifest, velocity, pressure, force, workers, budget, evaluations, cap, velocity_floor, pressure_floor, backend, output @ ..] =
        rest
    else {
        return Err(format!("expected eleven or twelve arguments; {USAGE}"));
    };
    let request = Request {
        velocity_samples: parse(velocity)?,
        pressure_samples: parse(pressure)?,
        force_samples: parse(force)?,
        workers: parse(workers)?,
        root_budget: parse(budget)?,
        max_reference_evaluations: parse(evaluations)?,
        cap: parse(cap)?,
        velocity_floor: velocity_floor
            .parse()
            .map_err(|_| "invalid velocity relative floor")?,
        pressure_floor: pressure_floor
            .parse()
            .map_err(|_| "invalid pressure relative floor")?,
        backend: backend.to_owned(),
    };
    Ok((
        std::path::PathBuf::from(manifest),
        request,
        output.first().map(std::path::PathBuf::from),
    ))
}

fn parse(value: &str) -> Result<usize, String> {
    value.parse().map_err(|_| "invalid integer argument".into())
}

fn parse_backend(name: &str) -> Result<FftBackend, String> {
    match name {
        "owned-radix" => Ok(FftBackend::OwnedRadix),
        "rustfft-6.4.1-avx-avx2-fma" => Ok(FftBackend::RustFft6_4_1AvxFma),
        _ => Err(format!("unsupported FFT backend {name:?}")),
    }
}

fn layout(dimension: usize) -> Result<Layout, String> {
    Layout::new([dimension; 3]).map_err(model::debug)
}

/// Bound and strongly bind every ledger input before any reservation is read:
/// power-of-two sample dimensions inside the admitted range, a bounded worker
/// count and a bounded identity length, rejected before arithmetic preflight.
fn inputs(
    domain: Domain,
    request: &Request,
    backend: FftBackend,
    identity_len: usize,
) -> Result<ledger::AdmissionInputs, String> {
    for (name, dimension) in [
        ("velocity", request.velocity_samples),
        ("pressure", request.pressure_samples),
        ("force", request.force_samples),
    ] {
        if !dimension.is_power_of_two() || !(8..=4096).contains(&dimension) {
            return Err(format!(
                "sample dimension {name}={dimension} is outside bounded admission 8..=4096 (power of two)"
            ));
        }
    }
    if request.workers == 0 || request.workers > 256 {
        return Err("worker count is outside bounded admission 1..=256".into());
    }
    if identity_len == 0 || identity_len > provenance::MAX_IDENTITY_BYTES {
        return Err("identity length is outside bounded admission".into());
    }
    Ok(ledger::AdmissionInputs {
        source: domain,
        velocity_samples: layout(request.velocity_samples)?,
        pressure_samples: layout(request.pressure_samples)?,
        force_samples: layout(request.force_samples)?,
        workers: request.workers,
        backend,
        root_budget: request.root_budget,
        identity_len,
    })
}

fn admitted_profile(manifest: &model::Manifest) -> Result<(), String> {
    if manifest.evolution.method != "cox-matthews" {
        return Err("unsupported evolution method (only cox-matthews is admitted)".into());
    }
    if manifest.evolution.case_sha256 != nsbu_benchmarks::CASE_SHA256 {
        return Err(
            "unsupported case profile (only the frozen similarity-mms-v2 case is admitted)"
                .into(),
        );
    }
    if manifest.evolution.lengths != [1.0; 3] || manifest.evolution.viscosity != 1.0 {
        return Err("only the unit-cube unit-viscosity v2 geometry is admitted".into());
    }
    if manifest.comparison_kind == ComparisonKind::MethodDiagnostic {
        return Err("unsupported comparison profile (method-diagnostic)".into());
    }
    if let Some(profile) = &manifest.profile {
        if !profile.matches_identity(&manifest.identity) {
            return Err("snapshot profile binding does not match the exact identity".into());
        }
    }
    bind_identity_fields(&manifest.identity, manifest)
}

fn identity_value<'a>(identity: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    identity
        .split(';')
        .find_map(|entry| entry.strip_prefix(prefix.as_str()))
        .ok_or_else(|| format!("snapshot identity carries no {key} field"))
}

fn bind_identity_fields(identity: &str, manifest: &model::Manifest) -> Result<(), String> {
    let field = |key: &str| identity_value(identity, key);
    if field("case")? != manifest.evolution.case_sha256 {
        return Err("snapshot identity case field differs from the manifest case".into());
    }
    if field("retained")?
        != manifest
            .dimensions
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
    {
        return Err("snapshot identity retained field differs from the manifest grid".into());
    }
    if field("provider")?.is_empty() {
        return Err("snapshot identity carries no force provider field".into());
    }
    Ok(())
}

fn clock_from(manifest: &model::Manifest) -> Result<(TickClock, [f64; 2]), String> {
    let target = manifest.evolution.clock_target;
    let elapsed = manifest.elapsed;
    let remaining = target
        .checked_sub(elapsed)
        .filter(|value| *value > 0)
        .ok_or("endpoint snapshots with zero remaining ticks are unsupported")?;
    let clock =
        TickClock::restore(manifest.evolution.quantum_exponent, target, elapsed, remaining)
            .map_err(model::debug)?;
    let time = nsbu_benchmarks::time::BenchmarkTime::new(clock).map_err(|_| {
        "snapshot clock does not carry the exact v2 time identity (physical target 1/128)".to_owned()
    })?;
    Ok((clock, time.rounding_estimates()))
}

fn execute(
    mode: Mode,
    manifest_path: &Path,
    request: Request,
    output: Option<&Path>,
) -> Result<String, String> {
    request.validate()?;
    let raw_text = provenance::read_text(manifest_path, provenance::MAX_MANIFEST_BYTES)?;
    let raw = provenance::parse_strict(&raw_text)?;
    if raw.get("schema").and_then(Value::as_str) != Some(provenance::INPUT_SCHEMA) {
        return Err(format!(
            "strict provenance refusal: manifest schema is not {:?}",
            provenance::INPUT_SCHEMA
        ));
    }
    let manifest = decode::read_external_reference_manifest(manifest_path)?;
    provenance::bind_snapshot(
        manifest_path.parent().unwrap_or(Path::new(".")),
        &raw,
        &manifest,
    )?;
    admitted_profile(&manifest)?;
    let (clock, rounding) = clock_from(&manifest)?;
    let backend = parse_backend(&request.backend)?;
    if mode == Mode::Run {
        backend.ensure_available().map_err(model::debug)?;
    }
    let domain = manifest.domain()?;
    let inputs = inputs(domain, &request, backend, manifest.identity.len())?;
    let (ledger, work) = observer::ReferenceObserver::preflight(inputs).map_err(model::debug)?;
    if work.reference_evaluations > request.max_reference_evaluations {
        return Err(format!(
            "work preflight refusal: {} reference evaluations exceed cap {}",
            work.reference_evaluations, request.max_reference_evaluations
        ));
    }
    let binding = report::binding_value(&manifest, &ledger, &work, &request.value(), rounding);
    let binding = insert(
        binding,
        "external_source_bindings",
        source_bind::included_bindings()?,
    );
    if mode == Mode::Preflight {
        return finish(
            insert(binding, "fits", json!(ledger.total_bytes <= request.cap)),
            output,
        );
    }
    observe_and_finish(
        &manifest,
        clock,
        backend,
        inputs,
        &ledger,
        &request,
        binding,
        output,
    )
}

#[allow(clippy::too_many_arguments)]
fn observe_and_finish(
    manifest: &model::Manifest,
    clock: TickClock,
    backend: FftBackend,
    inputs: ledger::AdmissionInputs,
    ledger: &ledger::ObserverLedger,
    request: &Request,
    mut binding: Value,
    output: Option<&Path>,
) -> Result<String, String> {
    if ledger.total_bytes > request.cap {
        return Err(format!(
            "resource preflight refusal: admitted {} exceeds cap {}",
            ledger.total_bytes, request.cap
        ));
    }
    let snapshot = decode::load(manifest)?;
    let catalog = FftCatalog::new(backend, ledger.catalog_bytes).map_err(model::debug)?;
    let mut observer = observer::ReferenceObserver::new(inputs, &catalog, ledger, request.cap)
        .map_err(model::debug)?;
    let observations = observer
        .observe(
            clock,
            snapshot.coefficients.each_ref().map(Vec::as_slice),
            request.velocity_floor,
            request.pressure_floor,
            request.root_budget,
        )
        .map_err(model::debug)?;
    if !report::all_finite(&observations) {
        return Err("non-finite analytical-reference measurement".into());
    }
    let rehashed = cache::coefficient_sha256(snapshot.coefficients.each_ref().map(Vec::as_slice));
    if rehashed != manifest.coefficient_sha256 || rehashed != snapshot.coefficient_sha256 {
        return Err("captured coefficients changed during observation".into());
    }
    let clock_value = binding["clock"].clone();
    binding = insert(
        binding,
        "observations",
        report::observation_value(&observations, &rehashed, &clock_value),
    );
    finish(binding, output)
}

fn insert(mut value: Value, key: &str, extra: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_owned(), extra);
    }
    value
}

fn finish(value: Value, output: Option<&Path>) -> Result<String, String> {
    let text = report::pretty(&value)?;
    match output {
        Some(path) => {
            publication::publish(path, text.as_bytes())?;
            Ok(format!("published-create-only {}", path.display()))
        }
        None => Ok(text),
    }
}

#[cfg(test)]
mod arm_tests;
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod guard_tests;
#[cfg(test)]
mod coverage_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod inventory_tests;
#[cfg(test)]
mod known_value_tests;
#[cfg(test)]
mod schedule_tests;
#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod publication_fault_tests;
#[cfg(test)]
mod strict_tests;
#[cfg(test)]
mod surface_tests;
#[cfg(test)]
mod tests;
