//! Semantic validation of the digest-bound frozen plan, on top of the reviewed
//! digest admission in the decoder.
//!
//! A plan is never trusted because its digest matched: after the digest binds,
//! the plan text must parse strictly and every required field must be present,
//! correctly typed and semantically exact. The approved schema is closed, the
//! qualification/authorization flags are required with their exact permitted
//! values, the approved binary/watchdog/source artifact hashes are required by
//! name (an arbitrary `*_sha256` never satisfies them), the retained and
//! integration-force layouts bind the actual snapshot grid, the endpoint ticks
//! bind the snapshot clock target, the schedule binds the manifest evolution
//! segment-by-segment, every contiguous segment length is a multiple of its
//! step so the endpoint is landed on exactly, and every observer node must be
//! a scheduled landing point that includes the snapshot clock. A re-hashed
//! plan (digest recomputed by an attacker) still fails here; wrong types and
//! wrong strings are refused, never silently skipped.
use serde_json::Value;

use crate::{
    model::Manifest,
    provenance::{lower_hex, parse_strict},
};

const HEX40: usize = 40;
const HEX64: usize = 64;

/// The only approved plan schema. Any other schema string is refused; schema
/// presence alone is never sufficient admission.
pub(crate) const APPROVED_PLAN_SCHEMA: &str = "p10-n512-analytical-reference-plan-v1";

/// The artifact hashes every admitted plan must carry, required by exact
/// name. Other `*_sha256` keys are unapproved and refused outright.
pub(crate) const APPROVED_ARTIFACT_HASHES: [&str; 3] =
    ["binary_sha256", "watchdog_sha256", "source_sha256"];

/// The qualification/authorization flags every admitted plan must carry. The
/// only permitted value for each is `false`: a missing flag is refused, and a
/// present flag of any other type or value is refused.
pub(crate) const REQUIRED_FALSE_FLAGS: [&str; 3] =
    ["qualification", "launch_authorized", "run_authorized"];

pub(crate) fn field<'a>(fields: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// Require a present, non-empty string field, naming the exact failure mode so
/// a wrong-typed field is never reported as merely missing or empty.
fn text<'a>(plan: &'a Value, key: &str) -> Result<&'a str, String> {
    match plan.get(key) {
        None => Err(format!("semantic plan refusal: plan field {key:?} is missing")),
        Some(Value::String(value)) if !value.is_empty() => Ok(value),
        Some(Value::String(_)) => {
            Err(format!("semantic plan refusal: plan field {key:?} is empty"))
        }
        Some(_) => Err(format!(
            "semantic plan refusal: plan field {key:?} is present but is not a string"
        )),
    }
}

/// Require a present, unsigned-integer field, distinguishing missing from a
/// wrong-typed value.
fn u64_field(plan: &Value, key: &str) -> Result<u64, String> {
    match plan.get(key) {
        None => Err(format!("semantic plan refusal: plan field {key:?} is missing")),
        Some(value) => value.as_u64().ok_or_else(|| {
            format!("semantic plan refusal: plan field {key:?} is present but is not an integer")
        }),
    }
}

/// A required qualification/authorization flag: present, boolean, exactly
/// false. Missing or wrong-typed flags are refused, never defaulted.
fn require_false_flag(plan: &Value, key: &str) -> Result<(), String> {
    match plan.get(key) {
        None => Err(format!(
            "semantic plan refusal: required plan flag {key:?} is missing"
        )),
        Some(Value::Bool(false)) => Ok(()),
        Some(_) => Err(format!("semantic plan refusal: plan {key} must be false")),
    }
}

fn hex<'a>(plan: &'a Value, key: &str, length: usize) -> Result<&'a str, String> {
    let value = text(plan, key)?;
    if lower_hex(value, length) {
        Ok(value)
    } else {
        Err(format!(
            "semantic plan refusal: plan {key} is not {length} lowercase hexadecimal"
        ))
    }
}

fn optional_source<'a>(plan: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match plan.get(key) {
        None => Ok(None),
        Some(_) => hex(plan, key, HEX40).map(Some),
    }
}

/// Validate the digest-bound plan text against the decoded snapshot. Every
/// required field must carry its approved schema, exact source binding, bound
/// profile, required honest qualification flags, exactly the approved named
/// artifact hashes, exact layouts/endpoint, a schedule that binds the manifest
/// evolution and lands on the endpoint by step-multiple segments, and an
/// observer-node set of scheduled landing points containing the snapshot clock.
pub(crate) fn validate_plan(
    plan: &Value,
    identity: &str,
    manifest: &Manifest,
) -> Result<(), String> {
    let fields = crate::provenance::identity_fields_strict(identity)?;
    if text(plan, "schema")? != APPROVED_PLAN_SCHEMA {
        return Err(format!(
            "semantic plan refusal: plan schema is not the approved {APPROVED_PLAN_SCHEMA:?}"
        ));
    }
    bind_plan_source(plan, manifest, &fields)?;
    if text(plan, "profile")? != field(&fields, "profile").unwrap_or_default() {
        return Err("semantic plan refusal: plan profile differs from the identity profile field"
            .into());
    }
    bind_plan_artifacts(plan)?;
    for flag in REQUIRED_FALSE_FLAGS {
        require_false_flag(plan, flag)?;
    }
    bind_plan_semantics(plan, manifest, &fields)
}

fn bind_plan_source(plan: &Value, manifest: &Manifest, fields: &[(String, String)]) -> Result<(), String> {
    let declared = declared_plan_source(plan)?;
    if declared != manifest.source_commit {
        return Err(
            "semantic plan refusal: plan source commit differs from the snapshot source commit"
                .into(),
        );
    }
    if field(fields, "source") != Some(manifest.source_commit.as_str()) {
        return Err(
            "semantic plan refusal: snapshot identity source field differs from the snapshot source commit"
                .into(),
        );
    }
    for (plan_key, identity_key) in [
        ("production_source_commit", "production_source"),
        ("numerical_test_source_commit", "test_source"),
    ] {
        bind_extension_source(plan, fields, plan_key, identity_key)?;
    }
    Ok(())
}

fn declared_plan_source(plan: &Value) -> Result<&str, String> {
    match (
        optional_source(plan, "source_commit")?,
        optional_source(plan, "harness_commit_and_run_source")?,
    ) {
        (Some(commit), other)
            if other.is_some_and(|other| other != commit) =>
        {
            Err("semantic plan refusal: plan source_commit differs from its harness run source"
                .into())
        }
        (Some(commit), _) | (None, Some(commit)) => Ok(commit),
        (None, None) => {
            Err("semantic plan refusal: plan declares no source commit identity".into())
        }
    }
}

fn bind_extension_source(
    plan: &Value,
    fields: &[(String, String)],
    plan_key: &str,
    identity_key: &str,
) -> Result<(), String> {
    let Some(declared) = optional_source(plan, plan_key)? else {
        return Ok(());
    };
    match field(fields, identity_key) {
        Some(identity_value) if identity_value == declared => Ok(()),
        Some(_) => Err(format!(
            "semantic plan refusal: plan {plan_key} differs from the identity {identity_key} field"
        )),
        None => Err(format!(
            "semantic plan refusal: identity carries no {identity_key} field for the plan {plan_key}"
        )),
    }
}

/// The exact named binary/watchdog/source artifact hashes are each required as
/// 64 lowercase hexadecimal; no arbitrary `*_sha256` key can substitute for
/// them, and any unapproved `*_sha256` key is itself refused.
fn bind_plan_artifacts(plan: &Value) -> Result<(), String> {
    for key in APPROVED_ARTIFACT_HASHES {
        if plan.get(key).is_none() {
            return Err(format!(
                "semantic plan refusal: required artifact hash field {key:?} is missing"
            ));
        }
        hex(plan, key, HEX64)?;
    }
    if let Some(object) = plan.as_object() {
        if let Some(unapproved) = object
            .keys()
            .find(|key| key.ends_with("_sha256") && !APPROVED_ARTIFACT_HASHES.contains(&key.as_str()))
        {
            return Err(format!(
                "semantic plan refusal: plan carries the unapproved artifact hash field {unapproved:?}"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Segment {
    from: u128,
    until: u128,
    step: u128,
}

/// Exact layouts, endpoint, evolution-bound schedule and approved observer
/// nodes: the schedule must equal the manifest evolution schedule segment by
/// segment up to the snapshot clock, extend it contiguously to exactly the
/// endpoint with each segment length a multiple of its step, and every
/// observer node must be a landing point of that schedule.
fn bind_plan_semantics(
    plan: &Value,
    manifest: &Manifest,
    fields: &[(String, String)],
) -> Result<(), String> {
    let endpoint = bind_plan_geometry(plan, manifest, fields)?;
    let segments = parse_plan_segments(plan)?;
    bind_schedule_to_evolution(&segments, manifest)?;
    bind_schedule_contiguity(&segments, endpoint)?;
    bind_plan_nodes(plan, &segments, endpoint, manifest.elapsed)
}

fn bind_plan_geometry(
    plan: &Value,
    manifest: &Manifest,
    fields: &[(String, String)],
) -> Result<u128, String> {
    let retained = u64_field(plan, "retained_layout")?;
    if manifest.dimensions.first().map(|value| *value as u64) != Some(retained) {
        return Err(
            "semantic plan refusal: plan retained layout differs from the snapshot grid".into(),
        );
    }
    let force = u64_field(plan, "integration_force_layout")?;
    if manifest
        .evolution
        .integration_force_dimensions
        .first()
        .map(|value| *value as u64)
        != Some(force)
    {
        return Err("semantic plan refusal: plan integration force layout differs from the snapshot evolution".into());
    }
    if let Some(method) = field(fields, "method") {
        if method != manifest.evolution.method {
            return Err(
                "semantic plan refusal: identity method field differs from the manifest method"
                    .into(),
            );
        }
    }
    let ticks = u64_field(plan, "endpoint_ticks")?;
    let endpoint = u128::from(ticks);
    if field(fields, "endpoint").map(str::parse::<u128>) != Some(Ok(endpoint)) {
        return Err(
            "semantic plan refusal: plan endpoint differs from the identity endpoint field".into(),
        );
    }
    if endpoint != manifest.evolution.clock_target {
        return Err(
            "semantic plan refusal: plan endpoint differs from the snapshot clock target".into(),
        );
    }
    if manifest.elapsed >= endpoint {
        return Err(
            "semantic plan refusal: snapshot clock is at or beyond the frozen plan endpoint; \
             it carries no remaining ticks inside the planned window"
                .into(),
        );
    }
    Ok(endpoint)
}

fn parse_plan_segments(plan: &Value) -> Result<Vec<Segment>, String> {
    let segments = plan
        .get("schedule")
        .and_then(Value::as_array)
        .ok_or("semantic plan refusal: plan carries no schedule array")?;
    if segments.is_empty() {
        return Err("semantic plan refusal: plan schedule is empty".into());
    }
    segments
        .iter()
        .map(|segment| {
            Ok(Segment {
                from: u128::from(segment_tick(segment, "from_inclusive")?),
                until: u128::from(segment_tick(segment, "until_exclusive")?),
                step: u128::from(segment_tick(segment, "step_ticks")?),
            })
        })
        .collect()
}

fn segment_tick(segment: &Value, key: &str) -> Result<u64, String> {
    let value = segment.get(key).ok_or_else(|| {
        format!("semantic plan refusal: plan schedule segment field {key:?} is missing")
    })?;
    value.as_u64().ok_or_else(|| {
        format!("semantic plan refusal: plan schedule segment field {key:?} is present but is not an integer")
    })
}

/// The plan schedule must replay the manifest's own (already decoder-validated)
/// evolution schedule exactly, segment by segment, before it may extend beyond
/// the snapshot clock: the snapshot was integrated under that schedule, so a
/// plan whose prefix disagrees describes a different run.
fn bind_schedule_to_evolution(segments: &[Segment], manifest: &Manifest) -> Result<(), String> {
    let evolution = "semantic plan refusal: plan schedule does not bind the manifest evolution schedule";
    if segments.len() < manifest.evolution.schedule.len() {
        return Err(format!("{evolution} (fewer segments than the integrated evolution)"));
    }
    for (segment, actual) in segments.iter().zip(&manifest.evolution.schedule) {
        if segment.from != actual.from_inclusive
            || segment.until != actual.until_exclusive
            || segment.step != actual.step_ticks
        {
            return Err(format!("{evolution} (a segment disagrees with the integrated schedule)"));
        }
    }
    Ok(())
}

/// Contiguous from rest to exactly the endpoint, every segment bounded by the
/// endpoint, and every segment length a multiple of its step so the segment
/// lands on its own exclusive bound exactly.
fn bind_schedule_contiguity(segments: &[Segment], endpoint: u128) -> Result<(), String> {
    let mut previous_until = 0_u128;
    for segment in segments {
        if segment.step == 0
            || segment.from != previous_until
            || segment.from >= segment.until
            || segment.until > endpoint
        {
            return Err(
                "semantic plan refusal: plan schedule segments are not a complete, contiguous, endpoint-bounded schedule"
                    .into(),
            );
        }
        if !segment.until.saturating_sub(segment.from).is_multiple_of(segment.step) {
            return Err(
                "semantic plan refusal: plan schedule segment length is not a multiple of its step"
                    .into(),
            );
        }
        previous_until = segment.until;
    }
    if previous_until != endpoint {
        return Err("semantic plan refusal: plan schedule does not end exactly at the endpoint".into());
    }
    Ok(())
}

fn scheduled_landing(segments: &[Segment], tick: u128) -> bool {
    segments.iter().any(|segment| {
        segment.from <= tick
            && (tick == segment.until || (tick < segment.until && (tick - segment.from).is_multiple_of(segment.step)))
    })
}

/// Every observer node must be exactly approved: strictly ascending inside the
/// endpoint, a landing point of the schedule (a declared step boundary), and
/// the set must contain the snapshot clock.
fn bind_plan_nodes(plan: &Value, segments: &[Segment], endpoint: u128, elapsed: u128) -> Result<(), String> {
    let nodes = plan
        .get("observer_nodes")
        .and_then(Value::as_array)
        .ok_or("semantic plan refusal: plan carries no observer_nodes array")?;
    if nodes.is_empty() {
        return Err("semantic plan refusal: plan observer_nodes is empty".into());
    }
    let mut previous: Option<u128> = None;
    let mut contains = false;
    for node in nodes {
        let tick = u128::from(node.as_u64().ok_or(
            "semantic plan refusal: plan observer node is present but is not an integer",
        )?);
        let exceeds = previous.is_some_and(|previous| tick <= previous) || tick > endpoint;
        if exceeds {
            return Err(
                "semantic plan refusal: plan observer nodes are not strictly ascending inside the endpoint"
                    .into(),
            );
        }
        if !scheduled_landing(segments, tick) {
            return Err(format!(
                "semantic plan refusal: plan observer node {tick} is not a scheduled landing point of the plan schedule"
            ));
        }
        contains |= tick == elapsed;
        previous = Some(tick);
    }
    if !contains {
        return Err("semantic plan refusal: snapshot clock is not a declared observer node".into());
    }
    Ok(())
}

/// Parse a digest-bound plan file body for semantic validation.
pub(crate) fn parse_plan_body(bytes: &[u8]) -> Result<Value, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "semantic plan refusal: plan file is not valid UTF-8".to_owned())?;
    parse_strict(text)
}
