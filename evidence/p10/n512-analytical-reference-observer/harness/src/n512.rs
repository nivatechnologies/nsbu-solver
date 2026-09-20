//! Resource-only N512 ledger: pure arithmetic preflight for the live profile
//! from the copied clock record. It checks no execution availability and
//! allocates no state, observer, catalog, force table, cache or snapshot; the
//! nominal headroom comparison is an estimate and a fresh actual-memory
//! measurement is still required before any N512 observation.
//!
//! Even in this arithmetic-only mode the record binds structurally before any
//! ledger byte is reported: exact frozen case, retained 512 ledger grid,
//! non-empty provider, structured schema (record field and identity field in
//! agreement), a 40-hexadecimal identity `source` and the record's canonical
//! 40-hexadecimal `source_commit` string — present, string-typed and equal to
//! that source, never silently skipped when wrong-typed — the cox-matthews plan
//! method, a clock strictly inside the plan endpoint window, positive
//! epoch/accepted-step counts and a 64-hexadecimal `state_sha256` artifact hash.
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    identity_value, inputs,
    ledger::{ObserverLedger, WorkLedger},
    model, observer, parse_backend, plan, provenance, record,
    report::{self, SCOPE, SCHEMA},
    Request, LEDGER_BACKEND_NAME, LEDGER_DIMENSIONS, LEDGER_LENGTHS, LEDGER_VISCOSITY,
    LEDGER_WORKERS,
};

const BACCUS_512_GIB_NOMINAL_BYTES: u128 = 549_755_813_888;

/// The embedded snapshot identity beside its strictly parsed field list.
type IdentityFields<'a> = (&'a str, Vec<(String, String)>);

pub(crate) fn n512_ledger(
    record_path: &Path,
    request: Request,
    output: Option<&Path>,
) -> Result<String, String> {
    request.validate()?;
    let record_text = provenance::read_text(record_path, provenance::MAX_RECORD_BYTES)?;
    let record = provenance::parse_strict(&record_text)?;
    let (identity, fields) = bind_clock_identity(&record)?;
    record::validate_record(&record, &fields)?;
    let domain = nsbu_solver::domain::Domain::new(LEDGER_DIMENSIONS, LEDGER_LENGTHS, LEDGER_VISCOSITY)
        .map_err(model::debug)?;
    let backend = parse_backend(LEDGER_BACKEND_NAME)?;
    let inputs = inputs(domain, &request, backend, identity.len())?;
    let (ledger, work) = observer::ReferenceObserver::preflight(inputs).map_err(model::debug)?;
    if work.reference_evaluations > request.max_reference_evaluations {
        return Err(format!(
            "work preflight refusal: {} reference evaluations exceed cap {}",
            work.reference_evaluations, request.max_reference_evaluations
        ));
    }
    let total = u128::try_from(ledger.total_bytes).map_err(model::debug)?;
    let value = ledger_output_value(record_path, identity, &record, &fields, &request, &ledger, &work, total)?;
    crate::finish(value, output)
}

/// Bind the clock record to a frozen similarity-mms-v2 N512 capture: the
/// record must be a non-resumable, unqualified capture whose embedded identity
/// carries the frozen case, the N512 retained grid and a real force provider.
fn bind_clock_identity(record: &Value) -> Result<IdentityFields<'_>, String> {
    let identity = record["identity"]
        .as_str()
        .ok_or("clock record carries no identity string")?;
    if record["resumable"].as_bool() != Some(false) {
        return Err("clock record does not declare a non-resumable captured state".into());
    }
    if record["qualification"].as_bool() != Some(false) {
        return Err("clock record does not declare qualification=false".into());
    }
    if identity_value(identity, "case")? != nsbu_benchmarks::CASE_SHA256 {
        return Err(
            "clock record identity case differs from the frozen similarity-mms-v2 case".into(),
        );
    }
    if identity_value(identity, "retained")? != LEDGER_DIMENSIONS[0].to_string() {
        return Err("clock record identity retained grid differs from the N512 ledger profile".into());
    }
    if identity_value(identity, "provider")?.is_empty() {
        return Err("clock record identity carries no force provider field".into());
    }
    Ok((identity, provenance::identity_fields_strict(identity)?))
}

#[allow(clippy::too_many_arguments)]
fn ledger_output_value(
    record_path: &Path,
    identity: &str,
    record: &Value,
    fields: &[(String, String)],
    request: &Request,
    ledger: &ObserverLedger,
    work: &WorkLedger,
    total: u128,
) -> Result<Value, String> {
    let headroom = BACCUS_512_GIB_NOMINAL_BYTES.checked_sub(total);
    Ok(json!({
        "schema": SCHEMA,
        "mode": "n512-ledger",
        "scope": SCOPE,
        "qualification": false,
        "accepted_windows": 0,
        "basis": "arithmetic-preflight-only:no-availability-check:no-allocation-of-state-observer-catalog-force-table-cache-snapshot",
        "clock_record": record_path.display().to_string(),
        "snapshot_identity_bytes": identity.len(),
        "identity_binding": {
            "case_sha256": nsbu_benchmarks::CASE_SHA256,
            "source_commit": plan::field(fields, "source").unwrap_or_default(),
            "record_source_commit": record["source_commit"].as_str().expect("validated canonical source_commit"),
            "profile": plan::field(fields, "profile").unwrap_or_default(),
            "method": plan::field(fields, "method").unwrap_or_default(),
            "record_schema": record["schema"].as_str().expect("validated schema"),
            "identity_schema": plan::field(fields, "schema").unwrap_or_default(),
            "endpoint": plan::field(fields, "endpoint").unwrap_or_default(),
            "record_clock": record["clock"],
            "epoch": record["epoch"],
            "accepted_steps": record["accepted_steps"],
            "state_sha256": record["state_sha256"],
            "coefficient_bytes": record["coefficient_bytes"],
        },
        "identity_fields": fields
            .iter()
            .map(|(field, value)| json!({"field": field, "value": value}))
            .collect::<Vec<_>>(),
        "profile": {
            "dimensions": LEDGER_DIMENSIONS,
            "lengths": LEDGER_LENGTHS,
            "viscosity": LEDGER_VISCOSITY,
            "workers": LEDGER_WORKERS,
            "backend": LEDGER_BACKEND_NAME,
        },
        "budget": {
            "baccus_512_gib_nominal_bytes": BACCUS_512_GIB_NOMINAL_BYTES,
            "total_bytes": total,
            "headroom_vs_nominal_estimate_bytes": headroom,
            "fits_baccus_512_gib_nominal_estimate": headroom.is_some(),
            "nominal_note": "estimate only; the live-array ledger term was corrected upward by 16106127360 bytes to match the actual N512 observer allocations, and a fresh actual-memory measurement of the target host is still required before any N512 observation",
        },
        "requested": request.value(),
        "floors_note": "relative floors are run-path inputs and are fixed in arithmetic-only ledger mode",
        "fits": ledger.total_bytes <= request.cap,
        "ledger": report::ledger_value(ledger),
        "work": report::work_value(work, request.root_budget),
        "external_source_bindings": crate::source_bind::included_bindings()?,
    }))
}

/// The corrected live-array comparison term is exactly
/// `2*largest + 2*velocity + 2*pressure` f64 slots; exposed so the regression
/// guard can pin the 15 GiB correction against a future refactor.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn expected_comparison_array_bytes(velocity: usize, pressure: usize) -> usize {
    let largest = velocity.max(pressure);
    (2 * largest + 2 * velocity + 2 * pressure) * std::mem::size_of::<f64>()
}
