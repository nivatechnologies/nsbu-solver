//! N256/M512 completed-state lineage intake gate.
//!
//! The reviewed capture preparation (`p10-n256-m512-endpoint-capture-preparation-v1`,
//! evidence/p10/n256-m512-endpoint-prep-20260914) records
//! `coarse_trajectory_executed: false` and `endpoint_attempts_executed: 0`.
//! No completed, independently evolved from-rest N256/M512 lineage exists in
//! the vault, so runtime admission must refuse until one exists.
//!
//! The gate deliberately separates two things an intake cannot conflate:
//!
//! * **Metadata availability** — the declared artifacts are present and are
//!   schema-conformant. The clock-zero rest is the exact `p10-avx-n384-rest-v1`
//!   metadata-only JSON the capture writer actually emits (`state_payload:false`,
//!   no binary payload); the positive committed states are writer-schema binary
//!   snapshots whose full identity *bytes* (not just their length) and clock
//!   envelope are read within a bounded header probe. This never reads, hashes
//!   or reconstructs a full state payload.
//! * **Completed independent lineage** — the endpoint was independently evolved
//!   from the declared rest. That can only be asserted by cross-binding the
//!   intake to a *trusted, closed, independently reviewed* lineage receipt that
//!   pins the source/binary/plan hashes, the rest file hash, every committed
//!   step's file/coefficient hash and the endpoint. A self-attested intake
//!   (`from_rest` boolean + sparse headers + hex-shaped hashes) is metadata, not
//!   evidence of history, so it can never satisfy this.
//!
//! `reviewed_anchor()` returns `None` because the reviewed preparation records
//! no executed coarse trajectory: runtime admission therefore stays refused.
//! The verifier is not vacuous — it is exercised against synthetic receipts over
//! synthetic fixtures in the test module, and it refuses forged/same-length
//! identities, changed hashes and non-rest-schema artifacts there.

use crate::contract::{
    identity_field_equals, required_clocks, steps_through, CASE_SHA256, CLOCK_TARGET, COARSE_HOST,
    COARSE_PROFILE, COARSE_STATE_BYTES, COMPARISON_ENDPOINT,
};
use crate::decode::{is_hex, read_bounded};
use crate::model::{
    debug, ClockHeader, LineageBinding, LineageIntake, Manifest, RestArtifact, RestRecord,
    StateRecord, LINEAGE_SCHEMA, REST_BALANCE, REST_OBSERVATION_STATUS, REST_SCHEMA,
};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAGIC: &[u8; 12] = b"P10AVXSNAP1\0";
const MAX_INTAKE_BYTES: u64 = 64 * 1024;
const MAX_REST_BYTES: u64 = 4 * 1024;
const MAX_IDENTITY_BYTES: usize = 16 * 1024;
const HEADER_CLOCK_BYTES: u64 = 4 * 16;

pub(crate) const LINEAGE_ABSENT: &str = "n256_m512_lineage_absent";
pub(crate) const LINEAGE_UNVERIFIED: &str = "n256_m512_lineage_unverified";

#[derive(Debug)]
pub(crate) struct Intake {
    pub(crate) value: LineageIntake,
    pub(crate) endpoint_file_sha256: String,
}

/// One committed step as pinned by an independently reviewed closed lineage
/// receipt: the actual clock plus the full-file and coefficient SHA-256 of the
/// committed bundle's state payload. These are the values a self-attested intake
/// can claim but cannot itself make authoritative.
pub(crate) struct TrustedStep {
    pub(crate) clock: u128,
    pub(crate) coefficient_sha256: String,
    pub(crate) file_sha256: String,
}

/// A closed, independently reviewed completed-capture lineage receipt for the
/// N256/M512 endpoint. It is supplied out of band (compiled-in / pinned like
/// [`crate::fine_identity`]) and can never be satisfied by the intake's own
/// fields. It authenticates the source/binary/plan identity, the rest file hash,
/// every committed step's file/coefficient hash and the endpoint clock — the
/// evidence that turns a *metadata-present* intake into a *completed independent
/// lineage*.
pub(crate) struct TrustedLineage {
    pub(crate) identity: String,
    pub(crate) source_commit: String,
    pub(crate) plan_sha256: String,
    pub(crate) binary_sha256: String,
    pub(crate) rest_file_sha256: String,
    pub(crate) steps: Vec<TrustedStep>,
}

/// The independently reviewed closed N256/M512 completed-lineage receipt. There
/// is none: the reviewed capture preparation records `coarse_trajectory_executed:
/// false` and `endpoint_attempts_executed: 0`, so no endpoint was independently
/// evolved and no completed-capture receipt was ever reviewed. Runtime admission
/// is therefore refused rather than inferred from a self-attested intake.
pub(crate) fn reviewed_anchor() -> Option<TrustedLineage> {
    None
}

/// Read and verify the intake JSON against a coarse manifest, cross-bind the
/// reviewed identity, probe the declared artifacts with bounded reads only,
/// then require a trusted closed lineage receipt for completed-lineage admission.
pub(crate) fn admit_through_manifest(manifest: &Manifest) -> Result<Intake, String> {
    admit_through_manifest_with_anchor(manifest, reviewed_anchor().as_ref())
}

/// The coarse-manifest pipeline with an explicit lineage receipt. Runtime callers
/// pass `reviewed_anchor()` (currently `None`); tests pass a synthetic receipt
/// bound to fixture hashes to exercise the authentication path itself.
pub(crate) fn admit_through_manifest_with_anchor(
    manifest: &Manifest,
    anchor: Option<&TrustedLineage>,
) -> Result<Intake, String> {
    let binding = manifest.lineage.as_ref().ok_or(LINEAGE_ABSENT)?;
    let intake = read_intake(&binding.intake, Some(binding))?;
    cross_bind(&intake.value, manifest)?;
    if let Some(anchor) = anchor {
        if anchor.identity != manifest.identity {
            return Err("reviewed lineage identity does not match the coarse manifest".into());
        }
    }
    // The coarse manifest carries the reviewed identity fields; committed states
    // and the rest must carry exactly those identity bytes.
    probe_records(&intake.value, Some(manifest.identity.as_str()))?;
    bind_endpoint(manifest, &intake)?;
    verify_completed_lineage(&intake.value, anchor)?;
    Ok(intake)
}

/// Standalone admission for `admit` mode: verifies the intake JSON and probes the
/// declared artifacts with bounded reads, then requires a trusted closed lineage
/// receipt. With no reviewed receipt (the runtime reality) metadata presence is
/// reported as unverified and completed-lineage admission stays refused.
pub(crate) fn admit_standalone(path: &Path) -> Result<Intake, String> {
    admit_standalone_with_anchor(path, reviewed_anchor().as_ref())
}

pub(crate) fn admit_standalone_with_anchor(
    path: &Path,
    anchor: Option<&TrustedLineage>,
) -> Result<Intake, String> {
    let intake = read_intake(path, None)?;
    // With no manifest the reviewed identity is only known through the receipt.
    probe_records(&intake.value, anchor.map(|anchor| anchor.identity.as_str()))?;
    verify_completed_lineage(&intake.value, anchor)?;
    Ok(intake)
}

fn read_intake(path: &Path, binding: Option<&LineageBinding>) -> Result<Intake, String> {
    if !path.exists() {
        return Err(format!(
            "{LINEAGE_ABSENT}: no N256/M512 lineage intake exists at {}",
            path.display()
        ));
    }
    let bytes = read_bounded(
        path,
        MAX_INTAKE_BYTES,
        "lineage intake exceeds 64 KiB bound",
    )?;
    if let Some(binding) = binding {
        if format!("{:x}", Sha256::digest(&bytes)) != binding.intake_sha256 {
            return Err("lineage intake SHA-256 does not match reviewed manifest".into());
        }
    }
    let value: LineageIntake = serde_json::from_slice(&bytes).map_err(|_| {
        format!(
            "{LINEAGE_ABSENT}: invalid N256/M512 lineage intake schema at {}",
            path.display()
        )
    })?;
    validate_structure(&value)?;
    let endpoint_file_sha256 = endpoint_record(&value)?.file_sha256.clone();
    Ok(Intake {
        value,
        endpoint_file_sha256,
    })
}

fn endpoint_record(intake: &LineageIntake) -> Result<&StateRecord, String> {
    intake
        .states
        .iter()
        .find(|record| record.clock == COMPARISON_ENDPOINT)
        .ok_or_else(|| "lineage intake omits the clock-4096 endpoint state".to_string())
}

fn validate_structure(intake: &LineageIntake) -> Result<(), String> {
    if intake.schema != LINEAGE_SCHEMA
        || intake.host != COARSE_HOST
        || intake.profile != COARSE_PROFILE
        || !intake.from_rest
        || !is_hex(&intake.source_commit, 40)
        || !is_hex(&intake.plan_sha256, 64)
        || !is_hex(&intake.binary_sha256, 64)
        || !is_hex(&intake.rest.file_sha256, 64)
        || intake.rest.path.is_relative()
    {
        return Err("invalid lineage intake binding".into());
    }
    let expected = required_clocks();
    let supplied: Vec<u128> = intake.states.iter().map(|record| record.clock).collect();
    if supplied.len() != expected.len() || supplied != expected {
        return Err("lineage intake clocks deviate from the closed h64/h128 schedule".into());
    }
    for record in &intake.states {
        if !is_hex(&record.coefficient_sha256, 64)
            || !is_hex(&record.file_sha256, 64)
            || record.path.is_relative()
            || steps_through(record.clock).is_err()
        {
            return Err("invalid lineage state record binding".into());
        }
    }
    Ok(())
}

fn cross_bind(intake: &LineageIntake, manifest: &Manifest) -> Result<(), String> {
    if intake.source_commit != manifest.source_commit
        || intake.plan_sha256 != manifest.plan_sha256
        || intake.profile != manifest.profile.value
        || !identity_field_equals(&manifest.identity, "source", &intake.source_commit)
        || !identity_field_equals(&manifest.identity, "case", CASE_SHA256)
    {
        return Err("lineage intake does not cross-bind the coarse manifest".into());
    }
    Ok(())
}

/// Bounded metadata probe over the actual artifacts. The clock-zero rest is the
/// exact metadata-only writer JSON; positive states are writer-schema binary
/// snapshots probed by header only. The reviewed identity, when known, is
/// compared byte-for-byte against the actual bytes stored in every artifact.
fn probe_records(intake: &LineageIntake, reviewed_identity: Option<&str>) -> Result<(), String> {
    let mut missing = Vec::new();
    let mut malformed = Vec::new();
    probe_rest(
        &intake.rest,
        reviewed_identity,
        &mut missing,
        &mut malformed,
    );
    for record in &intake.states {
        probe_state(record, reviewed_identity, &mut missing, &mut malformed);
    }
    if !malformed.is_empty() {
        return Err(format!(
            "malformed N256/M512 lineage artifacts: {}",
            malformed.join("; ")
        ));
    }
    if !missing.is_empty() {
        return Err(format!(
            "{LINEAGE_ABSENT}: {} declared N256/M512 committed states are absent: {}",
            missing.len(),
            missing.join(", ")
        ));
    }
    Ok(())
}

fn probe_rest(
    rest: &RestRecord,
    reviewed_identity: Option<&str>,
    missing: &mut Vec<String>,
    malformed: &mut Vec<String>,
) {
    match probe_rest_one(rest, reviewed_identity) {
        Ok(()) => {}
        Err(ProbeError::Absent) => missing.push(rest.path.display().to_string()),
        Err(ProbeError::Malformed(reason)) => malformed.push(reason),
    }
}

fn probe_state(
    record: &StateRecord,
    reviewed_identity: Option<&str>,
    missing: &mut Vec<String>,
    malformed: &mut Vec<String>,
) {
    match probe_state_one(record.path.as_path(), record.clock, reviewed_identity) {
        Ok(()) => {}
        Err(ProbeError::Absent) => missing.push(record.path.display().to_string()),
        Err(ProbeError::Malformed(reason)) => malformed.push(reason),
    }
}

fn probe_rest_one(rest: &RestRecord, reviewed_identity: Option<&str>) -> Result<(), ProbeError> {
    if !rest.path.exists() {
        return Err(ProbeError::Absent);
    }
    // The rest is a small metadata record, not a binary snapshot. Verify the
    // declared file hash against the actual bytes, then the exact writer schema
    // and identity. No state payload is read.
    let bytes = read_bounded(
        &rest.path,
        MAX_REST_BYTES,
        "rest record exceeds 4 KiB bound",
    )
    .map_err(|reason| ProbeError::Malformed(format!("{reason} at {}", rest.path.display())))?;
    if format!("{:x}", Sha256::digest(&bytes)) != rest.file_sha256 {
        return Err(ProbeError::Malformed(format!(
            "rest record SHA-256 does not match the intake at {}",
            rest.path.display()
        )));
    }
    let artifact: RestArtifact = serde_json::from_slice(&bytes).map_err(|_| {
        ProbeError::Malformed(format!(
            "rest record is not the exact {REST_SCHEMA} metadata record at {}",
            rest.path.display()
        ))
    })?;
    if artifact.schema != REST_SCHEMA
        || artifact.clock != 0
        || artifact.state_payload
        || artifact.observation_status != REST_OBSERVATION_STATUS
        || artifact.balance != REST_BALANCE
    {
        return Err(ProbeError::Malformed(format!(
            "rest record schema/status mismatch at {}",
            rest.path.display()
        )));
    }
    if let Some(identity) = reviewed_identity {
        if artifact.identity != identity {
            return Err(ProbeError::Malformed(format!(
                "rest record identity mismatch at {}",
                rest.path.display()
            )));
        }
    }
    Ok(())
}

fn probe_state_one(
    path: &Path,
    clock: u128,
    reviewed_identity: Option<&str>,
) -> Result<(), ProbeError> {
    let metadata = std::fs::metadata(path).map_err(|_| ProbeError::Absent)?;
    let header = read_bounded_header(path)
        .map_err(|reason| ProbeError::Malformed(format!("{reason} at {}", path.display())))?;
    let expected_len = 12_u64
        .checked_add(8)
        .and_then(|value| value.checked_add(header.identity_len))
        .and_then(|value| value.checked_add(HEADER_CLOCK_BYTES))
        .and_then(|value| value.checked_add(u64::try_from(COARSE_STATE_BYTES).unwrap_or(u64::MAX)))
        .and_then(|value| value.checked_add(32))
        .ok_or_else(|| ProbeError::Malformed("lineage state size overflow".into()))?;
    if metadata.len() != expected_len {
        return Err(ProbeError::Malformed(format!(
            "lineage state length mismatch at {}: expected {expected_len}, found {}",
            path.display(),
            metadata.len()
        )));
    }
    // Compare the actual stored identity bytes, not just their length: a
    // same-length wrong source/profile/case identity must be refused.
    if let Some(identity) = reviewed_identity {
        if header.identity.as_slice() != identity.as_bytes() {
            return Err(ProbeError::Malformed(format!(
                "identity mismatch at {}",
                path.display()
            )));
        }
    }
    let expected_steps = if clock == 0 {
        0
    } else {
        steps_through(clock)
            .map_err(|reason| ProbeError::Malformed(format!("{reason} at {}", path.display())))?
    };
    if header.clock.elapsed != clock
        || header.clock.target != CLOCK_TARGET
        || header.clock.epoch != expected_steps
        || header.clock.accepted_steps != expected_steps
    {
        return Err(ProbeError::Malformed(format!(
            "lineage state clock header mismatch at {}",
            path.display()
        )));
    }
    Ok(())
}

/// Distinguish metadata availability from a completed independent lineage. A
/// metadata-conformant intake is *not* sufficient evidence of history: without a
/// trusted closed receipt that pins every committed step and endpoint, admission
/// is refused. With a receipt, every declared hash and the endpoint must
/// authenticate against it.
fn verify_completed_lineage(
    intake: &LineageIntake,
    anchor: Option<&TrustedLineage>,
) -> Result<(), String> {
    let anchor = anchor.ok_or_else(|| {
        format!(
            "{LINEAGE_UNVERIFIED}: a self-attested intake (from_rest flag, bounded state \
             headers and hex-shaped hashes) cannot establish a completed independent N256/M512 \
             trajectory; the reviewed capture preparation records coarse_trajectory_executed=false \
             and no closed completed-lineage receipt exists, so runtime admission stays refused"
        )
    })?;
    if intake.source_commit != anchor.source_commit
        || intake.plan_sha256 != anchor.plan_sha256
        || intake.binary_sha256 != anchor.binary_sha256
        || intake.rest.file_sha256 != anchor.rest_file_sha256
    {
        return Err(
            "lineage intake does not authenticate against the reviewed lineage receipt".into(),
        );
    }
    let expected_clocks = required_clocks();
    if anchor.steps.len() != expected_clocks.len() {
        return Err("reviewed lineage receipt does not cover the closed schedule".into());
    }
    for (index, record) in intake.states.iter().enumerate() {
        let step = &anchor.steps[index];
        if step.clock != expected_clocks[index] || record.clock != expected_clocks[index] {
            return Err(format!("reviewed lineage step {index} clock mismatch"));
        }
        if step.coefficient_sha256 != record.coefficient_sha256
            || step.file_sha256 != record.file_sha256
        {
            return Err(format!(
                "committed step clock={} is not authenticated against the reviewed lineage receipt",
                record.clock
            ));
        }
    }
    let endpoint = anchor
        .steps
        .last()
        .ok_or("reviewed lineage receipt has no endpoint")?;
    if endpoint.clock != COMPARISON_ENDPOINT {
        return Err("reviewed lineage receipt endpoint clock mismatch".into());
    }
    Ok(())
}

enum ProbeError {
    Absent,
    Malformed(String),
}

struct HeaderProbe {
    identity_len: u64,
    identity: Vec<u8>,
    clock: ClockHeader,
}

fn read_bounded_header(path: &Path) -> Result<HeaderProbe, String> {
    let mut file = File::open(path).map_err(debug)?;
    let mut magic = [0_u8; 12];
    file.read_exact(&mut magic).map_err(debug)?;
    if magic != *MAGIC {
        return Err("snapshot magic mismatch".into());
    }
    let mut word = [0_u8; 8];
    file.read_exact(&mut word).map_err(debug)?;
    let identity_len = u64::from_le_bytes(word);
    if usize::try_from(identity_len).is_err() || identity_len as usize > MAX_IDENTITY_BYTES {
        return Err("snapshot identity length out of bound".into());
    }
    // Consume and return the identity bytes so the caller can compare them,
    // rather than reading then discarding them.
    let mut identity = vec![0_u8; usize::try_from(identity_len).unwrap_or(0)];
    file.read_exact(&mut identity).map_err(debug)?;
    let mut clock_bytes = [0_u8; 64];
    file.read_exact(&mut clock_bytes).map_err(debug)?;
    let clock = ClockHeader {
        elapsed: u128::from_le_bytes(clock_bytes[0..16].try_into().map_err(debug)?),
        target: u128::from_le_bytes(clock_bytes[16..32].try_into().map_err(debug)?),
        epoch: u128::from_le_bytes(clock_bytes[32..48].try_into().map_err(debug)?),
        accepted_steps: u128::from_le_bytes(clock_bytes[48..64].try_into().map_err(debug)?),
    };
    Ok(HeaderProbe {
        identity_len,
        identity,
        clock,
    })
}

/// Endpoint hash binding used before loading: the coarse manifest must carry
/// exactly the endpoint hashes recorded by the intake.
pub(crate) fn bind_endpoint(manifest: &Manifest, intake: &Intake) -> Result<(), String> {
    if manifest.coefficient_sha256 != endpoint_record(&intake.value)?.coefficient_sha256
        || manifest.file_sha256 != intake.endpoint_file_sha256
    {
        return Err("coarse manifest endpoint hashes do not match the lineage intake".into());
    }
    Ok(())
}
