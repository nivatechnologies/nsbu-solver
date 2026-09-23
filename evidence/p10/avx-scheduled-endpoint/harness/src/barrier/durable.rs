//! Durable-commit reading and the channel's durable REST-record binding.
//!
//! The channel binds its `rest` decision field to the SHA-256 of the ACTUAL
//! durable first (REST) record bytes and requires the reviewed run identity
//! verbatim in both the REST record and the armed step record.  A missing,
//! unreadable or foreign durable record refuses the decision BEFORE any
//! channel byte is consumed.  Nothing here is a model: every value is read
//! from the solver's own transactional publications.

use super::{token::is_hex64, ChannelConfig};
use crate::error::HarnessError;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const HEX64: usize = 64;
const STATE_MARKER: &str = "\"state_sha256\": \"";
const IDENTITY_MARKER: &str = "\"identity\": \"";

/// The capture profile publishes the durable first record either as
/// `rest.json` (offline-capture profiles) or as the clock-0 node record.
pub(super) const REST_RECORD: &str = if cfg!(capture_offline) {
    "rest.json"
} else {
    "node-0000/record.json"
};

/// (state hash, record text) of the committed bundle, or a fail-closed
/// refusal.  The bundle directory is created only by the transactional
/// publication, so its existence proves the durable commit.
pub(super) fn durable_state(
    output: &Path,
    attempt: usize,
    clock: u128,
) -> Result<(String, String), HarnessError> {
    let record = durable_bundle_record(output, attempt, clock)?;
    Ok((record_state_sha(&record)?, record))
}

/// The frozen public helper (kept byte-faithful for the frozen tests).
#[allow(dead_code)]
pub fn durable_state_sha(
    output: &Path,
    attempt: usize,
    clock: u128,
) -> Result<String, HarnessError> {
    record_state_sha(&durable_bundle_record(output, attempt, clock)?)
}

fn durable_bundle_record(
    output: &Path,
    attempt: usize,
    clock: u128,
) -> Result<String, HarnessError> {
    let bundle = output.join(format!("step-{attempt:03}-clock-{clock:04}"));
    if !bundle.is_dir() {
        return Err(HarnessError::Barrier("barrier_bundle_not_durable"));
    }
    fs::read_to_string(bundle.join("record.json"))
        .map_err(|_| HarnessError::Barrier("barrier_record_unreadable"))
}

pub fn record_state_sha(record: &str) -> Result<String, HarnessError> {
    let start = record
        .find(STATE_MARKER)
        .ok_or(HarnessError::Barrier("barrier_record_state_missing"))?
        + STATE_MARKER.len();
    let end = start + HEX64;
    let candidate = record
        .get(start..end)
        .ok_or(HarnessError::Barrier("barrier_record_state_missing"))?;
    if !is_hex64(candidate) || record[end..].find(STATE_MARKER).is_some() {
        return Err(HarnessError::Barrier("barrier_record_state_malformed"));
    }
    Ok(candidate.to_owned())
}

/// First `"identity": "<...>"` JSON string field of a harness record; the
/// harness writes identity through `artifact::json_string`, whose escaping is
/// impossible for the identity alphabet, so scanning to the next quote is
/// exact for every real record.
pub(super) fn record_identity(record: &str) -> Result<String, HarnessError> {
    let start = record
        .find(IDENTITY_MARKER)
        .ok_or(HarnessError::Barrier("barrier_record_identity_missing"))?
        + IDENTITY_MARKER.len();
    let end = record[start..]
        .find('"')
        .ok_or(HarnessError::Barrier("barrier_record_identity_missing"))?
        + start;
    Ok(record[start..end].to_owned())
}

/// The SHA-256 (hex) of a file's bytes.
pub(super) fn file_sha256(path: &Path) -> Result<String, HarnessError> {
    let bytes =
        fs::read(path).map_err(|_| HarnessError::Barrier("barrier_rest_record_unavailable"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(super::token::to_hex(&hasher.finalize()))
}

/// Channel-only durable binding, executed AFTER the armed step bundle is
/// durable and BEFORE the armed receipt is published or any channel byte is
/// read.  Returns the SHA-256 of the durable REST record bytes.  Every
/// deviation is a fail-closed refusal with a distinct reason:
///   * REST record missing/unreadable -> `barrier_rest_record_unavailable`;
///   * REST record carries a foreign identity -> `barrier_rest_record_identity_mismatch`;
///   * armed bundle record carries a foreign identity -> `barrier_armed_record_identity_mismatch`;
///   * the env-supplied `rest` hash differs from the durable bytes -> `barrier_channel_rest_unbound`.
pub(super) fn bind_channel_records(
    output: &Path,
    armed_record: &str,
    config: &ChannelConfig,
) -> Result<String, HarnessError> {
    let rest_path = output.join(REST_RECORD);
    if !rest_path.is_file() {
        return Err(HarnessError::Barrier("barrier_rest_record_unavailable"));
    }
    let rest_record = fs::read_to_string(&rest_path)
        .map_err(|_| HarnessError::Barrier("barrier_rest_record_unavailable"))?;
    if record_identity(&rest_record)? != config.reviewed_identity {
        return Err(HarnessError::Barrier(
            "barrier_rest_record_identity_mismatch",
        ));
    }
    if record_identity(armed_record)? != config.reviewed_identity {
        return Err(HarnessError::Barrier(
            "barrier_armed_record_identity_mismatch",
        ));
    }
    let rest_hash = file_sha256(&rest_path)?;
    if rest_hash != config.rest {
        return Err(HarnessError::Barrier("barrier_channel_rest_unbound"));
    }
    Ok(rest_hash)
}

pub fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(u64::MAX)
}
