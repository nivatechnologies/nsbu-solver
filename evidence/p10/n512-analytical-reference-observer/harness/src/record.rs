//! Strict semantic validation of the arithmetic-only ledger clock record: the
//! structured schema, the canonical source commit string, profile, method,
//! endpoint-bounded clock and artifact hash must all bind before a single
//! ledger byte is reported. Wrong types are refused, never skipped.
use serde_json::Value;

use crate::provenance::lower_hex;

use crate::plan::field;

const HEX40: usize = 40;
const HEX64: usize = 64;

pub(crate) fn validate_record(record: &Value, fields: &[(String, String)]) -> Result<(), String> {
    bind_record_schema(record, fields)?;
    bind_record_source(record, fields)?;
    bind_record_identity(fields)?;
    bind_record_clock(record, fields)?;
    bind_record_artifacts(record)
}

fn bind_record_schema(record: &Value, fields: &[(String, String)]) -> Result<(), String> {
    let schema = match record.get("schema") {
        None => return Err("ledger record refusal: record carries no structured schema field".into()),
        Some(Value::String(schema)) if !schema.is_empty() => schema.as_str(),
        Some(_) => {
            return Err(
                "ledger record refusal: record schema field is present but is not a string"
                    .into(),
            )
        }
    };
    if field(fields, "schema") != Some(schema) {
        return Err(
            "ledger record refusal: identity schema field differs from the record schema".into(),
        );
    }
    Ok(())
}

/// The ledger record must carry its source commit as the canonical 40-hex
/// string equal to the identity `source` field. A missing, wrong-typed or
/// non-canonical `source_commit` is always refused.
fn bind_record_source(record: &Value, fields: &[(String, String)]) -> Result<(), String> {
    let source = field(fields, "source")
        .ok_or("ledger record refusal: identity carries no source field")?;
    if !lower_hex(source, HEX40) {
        return Err(
            "ledger record refusal: identity source field is not 40 lowercase hexadecimal"
                .to_owned(),
        );
    }
    let commit = match record.get("source_commit") {
        None => {
            return Err(
                "ledger record refusal: record carries no canonical source_commit string"
                    .into(),
            )
        }
        Some(Value::String(commit)) => commit.as_str(),
        Some(_) => {
            return Err(
                "ledger record refusal: record source_commit is present but is not a string"
                    .into(),
            )
        }
    };
    if !lower_hex(commit, HEX40) || commit != source {
        return Err(
            "ledger record refusal: record source_commit differs from the identity source field"
                .into(),
        );
    }
    Ok(())
}

fn bind_record_identity(fields: &[(String, String)]) -> Result<(), String> {
    for key in ["case", "retained", "provider", "profile", "method", "endpoint"] {
        if field(fields, key).unwrap_or_default().is_empty() {
            return Err(format!(
                "ledger record refusal: identity carries no non-empty {key} field"
            ));
        }
    }
    if field(fields, "method") != Some("cox-matthews") {
        return Err("ledger record refusal: only the cox-matthews plan method is admitted".into());
    }
    for key in ["production_source", "test_source"] {
        if let Some(value) = field(fields, key) {
            if !lower_hex(value, HEX40) {
                return Err(format!(
                    "ledger record refusal: identity {key} field is not 40 lowercase hexadecimal"
                ));
            }
        }
    }
    Ok(())
}

fn bind_record_clock(record: &Value, fields: &[(String, String)]) -> Result<(), String> {
    let clock = positive_u64(record, "clock")?;
    let endpoint: u128 = field(fields, "endpoint")
        .unwrap_or_default()
        .parse()
        .map_err(|_| "ledger record refusal: identity endpoint field is not a clock to parse".to_owned())?;
    if u128::from(clock) >= endpoint {
        return Err(
            "ledger record refusal: record clock must lie strictly inside the plan window"
                .into(),
        );
    }
    for key in ["epoch", "accepted_steps"] {
        positive_u64(record, key)?;
    }
    Ok(())
}

fn bind_record_artifacts(record: &Value) -> Result<(), String> {
    let artifact = match record.get("state_sha256") {
        None => {
            return Err(
                "ledger record refusal: record carries no state_sha256 artifact hash".into(),
            )
        }
        Some(Value::String(artifact)) => artifact.as_str(),
        Some(_) => {
            return Err(
                "ledger record refusal: record state_sha256 is present but is not a string".into(),
            )
        }
    };
    if !lower_hex(artifact, HEX64) {
        return Err(
            "ledger record refusal: state_sha256 is not 64 lowercase hexadecimal".to_owned(),
        );
    }
    positive_u64(record, "coefficient_bytes")?;
    Ok(())
}

fn positive_u64(record: &Value, key: &str) -> Result<u64, String> {
    match record.get(key) {
        None => Err(format!("ledger record refusal: record {key} is missing")),
        Some(value) => value.as_u64().filter(|value| *value > 0).ok_or_else(|| {
            format!("ledger record refusal: record {key} is present but is not a positive integer")
        }),
    }
}
