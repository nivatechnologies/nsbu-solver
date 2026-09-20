//! Strict provenance parsing and semantic binding for snapshot manifests and
//! N512 clock records. Duplicate JSON keys are rejected anywhere in the text,
//! identity fields must be unique and complete, and the source, profile,
//! schema and plan fields must bind exactly to the decoded snapshot: schema to
//! the input contract, identity to the bytes embedded in the snapshot file,
//! plan to the plan file digest, and every provenance hash to its lowercase
//! hexadecimal form and to the decoded manifest.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

use crate::model::Manifest;

pub(crate) const INPUT_SCHEMA: &str = "p10-snapshot-comparison-input-v1";
pub(crate) const MAX_MANIFEST_BYTES: usize = 1 << 20;
pub(crate) const MAX_RECORD_BYTES: usize = 1 << 16;
pub(crate) const MAX_PLAN_BYTES: usize = 1 << 20;
pub(crate) const MAX_IDENTITY_BYTES: usize = 1 << 16;
const SNAPSHOT_MAGIC: &[u8] = b"P10AVXSNAP1\0";

/// Bounded streaming read: at most `maximum + 1` bytes ever leave the file
/// before any parsing, so an oversized or adversarial path cannot force an
/// unbounded allocation. The length refusal happens strictly before parsing.
pub(crate) fn read_bounded_bytes(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| format!("provenance read failure: {error:?}"))?;
    let mut bytes = Vec::new();
    file.take(u64::try_from(maximum).map_err(|_| "provenance refusal: unreadable bound")? + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("provenance read failure: {error:?}"))?;
    if bytes.len() > maximum {
        return Err(format!(
            "provenance refusal: {} exceeds {maximum} bytes",
            path.display()
        ));
    }
    Ok(bytes)
}

pub(crate) fn read_text(path: &Path, maximum: usize) -> Result<String, String> {
    let bytes = read_bounded_bytes(path, maximum)?;
    String::from_utf8(bytes).map_err(|_| "provenance refusal: file is not valid UTF-8".to_owned())
}

/// Parse JSON strictly: duplicate object keys are refused before any value is
/// trusted, and only a single top-level object is accepted.
pub(crate) fn parse_strict(text: &str) -> Result<Value, String> {
    reject_duplicate_keys(text)?;
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("strict provenance refusal: invalid JSON: {error}"))?;
    if !value.is_object() {
        return Err("strict provenance refusal: expected a JSON object".to_owned());
    }
    Ok(value)
}

enum Frame {
    Array,
    Object {
        keys: Vec<String>,
        expect_key: bool,
    },
}

/// Structural token scan that refuses any object repeating a key. The scan
/// state machine stays flat: every byte class is dispatched to one focused
/// handler so nesting depth never compounds.
pub(crate) fn reject_duplicate_keys(text: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    let mut index = 0_usize;
    let mut stack: Vec<Frame> = Vec::new();
    let mut complete = false;
    while index < bytes.len() {
        if is_json_space(bytes[index]) {
            index += 1;
            continue;
        }
        if complete {
            return Err("strict provenance refusal: trailing JSON content".into());
        }
        let (next, closed) = scan_step(bytes, index, &mut stack)?;
        index = next;
        complete |= closed && stack.is_empty();
    }
    finish_scan(&stack, complete)
}

fn is_json_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

fn finish_scan(stack: &[Frame], complete: bool) -> Result<(), String> {
    if !stack.is_empty() {
        return Err("strict provenance refusal: unterminated JSON structure".into());
    }
    if !complete {
        return Err("strict provenance refusal: empty JSON input".into());
    }
    Ok(())
}

/// Advance one non-whitespace byte class, returning the next index and whether
/// this step completed a container or top-level token.
fn scan_step(bytes: &[u8], index: usize, stack: &mut Vec<Frame>) -> Result<(usize, bool), String> {
    match bytes[index] {
        b'{' => {
            stack.push(Frame::Object {
                keys: Vec::new(),
                expect_key: true,
            });
            Ok((index + 1, false))
        }
        b'[' => {
            stack.push(Frame::Array);
            Ok((index + 1, false))
        }
        b'}' | b']' => {
            stack.pop().ok_or("strict provenance refusal: unbalanced JSON")?;
            Ok((index + 1, true))
        }
        b':' => set_object_key_state(stack, false).map(|()| (index + 1, false)),
        b',' => set_object_key_state(stack, true).map(|()| (index + 1, false)),
        b'"' => scan_and_note_string(bytes, index, stack),
        _ => scan_token(index, bytes).map(|end| (end, stack.is_empty())),
    }
}

fn set_object_key_state(stack: &mut [Frame], expect_key: bool) -> Result<(), String> {
    if let Some(Frame::Object { expect_key: slot, .. }) = stack.last_mut() {
        *slot = expect_key;
    }
    Ok(())
}

/// Scan a quoted string; when it occupies an object-key position, refuse any
/// escaped key and any key the enclosing object already carries.
fn scan_and_note_string(
    bytes: &[u8],
    index: usize,
    stack: &mut [Frame],
) -> Result<(usize, bool), String> {
    let end = scan_string(bytes, index)?;
    if matches!(stack.last(), Some(Frame::Object { expect_key: true, .. })) {
        note_object_key(stack, bytes, index, end)?;
    }
    Ok((end + 1, stack.is_empty()))
}

fn note_object_key(
    stack: &mut [Frame],
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<(), String> {
    let key = std::str::from_utf8(&bytes[start + 1..end])
        .map_err(|_| "strict provenance refusal: invalid UTF-8 object key")?;
    if key.contains('\\') {
        return Err(format!("strict provenance refusal: escaped object key {key:?}"));
    }
    let Some(Frame::Object { keys, expect_key }) = stack.last_mut() else {
        unreachable!("object frame just matched")
    };
    if keys.iter().any(|existing| existing == key) {
        return Err(format!("strict provenance refusal: duplicate JSON key {key:?}"));
    }
    keys.push(key.to_owned());
    *expect_key = false;
    Ok(())
}

fn scan_token(mut index: usize, bytes: &[u8]) -> Result<usize, String> {
    let start = index;
    while index < bytes.len() && !is_token_end(bytes[index]) {
        index += 1;
    }
    if index == start {
        return Err("strict provenance refusal: invalid JSON token".into());
    }
    Ok(index)
}

fn is_token_end(byte: u8) -> bool {
    is_json_space(byte) || matches!(byte, b',' | b']' | b'}' | b':' | b'{')
}

fn scan_string(bytes: &[u8], start: usize) -> Result<usize, String> {
    let mut cursor = start + 1;
    loop {
        if cursor >= bytes.len() {
            return Err("strict provenance refusal: unterminated string".into());
        }
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return Ok(cursor),
            _ => cursor += 1,
        }
    }
}

/// Parse a snapshot/clock identity into unique `key=value` fields; duplicate,
/// empty or delimiter-less fields are refused.
pub(crate) fn identity_fields_strict(identity: &str) -> Result<Vec<(String, String)>, String> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for (position, entry) in identity.split(';').enumerate() {
        if entry.is_empty() {
            return Err("strict provenance refusal: identity carries an empty field".into());
        }
        let (key, value) = split_identity_field(entry, position)?;
        if key.is_empty() {
            return Err("strict provenance refusal: identity field has an empty key".into());
        }
        if fields.iter().any(|(existing, _)| existing == key) {
            return Err(format!(
                "strict provenance refusal: identity carries duplicate field {key:?}"
            ));
        }
        fields.push((key.to_owned(), value.to_owned()));
    }
    Ok(fields)
}

fn split_identity_field(entry: &str, position: usize) -> Result<(&str, &str), String> {
    match entry.split_once('=') {
        Some(binding) => Ok(binding),
        // The reviewed identity format carries one free-form descriptor as
        // its leading field; every later field must be a `key=value` pair.
        None if position == 0 => Ok(("@descriptor", entry)),
        None => Err("strict provenance refusal: identity field carries no '='".into()),
    }
}

fn require_text<'a>(raw: &'a Value, key: &str) -> Result<&'a str, String> {
    raw.get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            format!("strict provenance refusal: manifest field {key:?} is missing or empty")
        })
}

pub(crate) fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn relative_name<'a>(raw: &'a Value, key: &str) -> Result<&'a str, String> {
    let name = require_text(raw, key)?;
    let path = Path::new(name);
    if path.is_absolute()
        || path.components().count() == 0
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "strict provenance refusal: {key} must stay inside the manifest directory"
        ));
    }
    Ok(name)
}

/// Bind every provenance field of the raw manifest text to the decoded snapshot.
pub(crate) fn bind_snapshot(
    directory: &Path,
    raw: &Value,
    manifest: &Manifest,
) -> Result<(), String> {
    bind_manifest_envelope(raw, manifest)?;
    let identity = require_text(raw, "identity")?;
    bind_identity_fields(identity, manifest)?;
    bind_plan_file(directory, raw, identity, manifest)?;
    bind_embedded_identity(directory, raw)
}

/// Bind the raw first bounded read of the manifest against the second bounded
/// read that the decoder deserialized: a file swapped between the two reads
/// must be refused, and every hash must be well-formed lowercase hex.
pub(crate) fn bind_manifest_envelope(raw: &Value, manifest: &Manifest) -> Result<(), String> {
    let schema = require_text(raw, "schema")?;
    if schema != INPUT_SCHEMA {
        return Err(format!(
            "strict provenance refusal: manifest schema {schema:?} is not {INPUT_SCHEMA:?}"
        ));
    }
    for (key, expected) in [
        ("identity", manifest.identity.as_str()),
        ("plan_sha256", manifest.plan_sha256.as_str()),
        ("file_sha256", manifest.file_sha256.as_str()),
        ("coefficient_sha256", manifest.coefficient_sha256.as_str()),
        ("source_commit", manifest.source_commit.as_str()),
        ("backend", manifest.backend.as_str()),
        ("execution", manifest.execution.as_str()),
    ] {
        let text = require_text(raw, key)?;
        if text != expected {
            return Err(format!(
                "strict provenance refusal: manifest field {key:?} differs from the decoded snapshot"
            ));
        }
    }
    let evolution_case = raw
        .get("evolution")
        .and_then(|evolution| evolution.get("case_sha256"))
        .and_then(Value::as_str)
        .ok_or("strict provenance refusal: manifest evolution carries no case_sha256")?;
    if evolution_case != manifest.evolution.case_sha256 {
        return Err(
            "strict provenance refusal: evolution case differs from the decoded snapshot".into(),
        );
    }
    if !lower_hex(require_text(raw, "source_commit")?, HEX40) {
        return Err(format!(
            "strict provenance refusal: source commit {:?} is not {HEX40} lowercase hex",
            require_text(raw, "source_commit")?
        ));
    }
    for key in ["plan_sha256", "coefficient_sha256", "file_sha256"] {
        if !lower_hex(require_text(raw, key)?, HEX64) {
            return Err(format!(
                "strict provenance refusal: {key} is not {HEX64} lowercase hexadecimal"
            ));
        }
    }
    let identity = require_text(raw, "identity")?;
    if identity.len() > MAX_IDENTITY_BYTES {
        return Err("strict provenance refusal: identity exceeds the bounded length".into());
    }
    Ok(())
}

const HEX40: usize = 40;
const HEX64: usize = 64;

fn bind_identity_fields(identity: &str, manifest: &Manifest) -> Result<(), String> {
    let fields = identity_fields_strict(identity)?;
    for key in ["case", "retained", "provider", "profile", "source"] {
        if !fields
            .iter()
            .any(|(field, value)| field == key && !value.is_empty())
        {
            return Err(format!(
                "strict provenance refusal: snapshot identity carries no non-empty {key} field"
            ));
        }
    }
    let field = |key: &str| {
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    };
    if field("source") != manifest.source_commit {
        return Err(
            "strict provenance refusal: snapshot identity source field differs from the snapshot source commit"
                .into(),
        );
    }
    if field("case") != manifest.evolution.case_sha256 {
        return Err("strict provenance refusal: identity case field differs from the manifest case"
            .into());
    }
    if field("retained")
        != manifest
            .dimensions
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
    {
        return Err(
            "strict provenance refusal: identity retained field differs from the manifest grid"
                .into(),
        );
    }
    Ok(())
}

fn bind_plan_file(
    directory: &Path,
    raw: &Value,
    identity: &str,
    manifest: &Manifest,
) -> Result<(), String> {
    let plan_name = relative_name(raw, "plan")?;
    let plan_bytes = read_bounded_bytes(&directory.join(plan_name), MAX_PLAN_BYTES)
        .map_err(|error| format!("strict provenance refusal: plan file unreadable: {error}"))?;
    if format!("{:x}", Sha256::digest(&plan_bytes)) != require_text(raw, "plan_sha256")? {
        return Err("strict provenance refusal: plan digest does not bind the plan file".into());
    }
    let plan = crate::plan::parse_plan_body(&plan_bytes)?;
    crate::plan::validate_plan(&plan, identity, manifest)
}

fn bind_embedded_identity(directory: &Path, raw: &Value) -> Result<(), String> {
    let identity = require_text(raw, "identity")?;
    let snapshot_name = relative_name(raw, "snapshot")?;
    let mut snapshot = fs::File::open(directory.join(snapshot_name)).map_err(|error| {
        format!("strict provenance refusal: snapshot header unreadable: {error:?}")
    })?;
    let mut header = [0_u8; 20];
    snapshot
        .read_exact(&mut header)
        .map_err(|_| "strict provenance refusal: snapshot header truncated".to_owned())?;
    if &header[..12] != SNAPSHOT_MAGIC {
        return Err("strict provenance refusal: snapshot magic differs".into());
    }
    let stored = u64::from_le_bytes(header[12..20].try_into().expect("header slice"));
    if stored as usize != identity.len() || stored as usize > MAX_IDENTITY_BYTES {
        return Err(
            "strict provenance refusal: embedded snapshot identity length differs".to_owned(),
        );
    }
    let mut embedded = vec![0_u8; identity.len()];
    snapshot
        .read_exact(&mut embedded)
        .map_err(|_| "strict provenance refusal: snapshot identity truncated".to_owned())?;
    if embedded != identity.as_bytes() {
        return Err(
            "strict provenance refusal: embedded snapshot identity differs from the manifest identity"
                .into(),
        );
    }
    Ok(())
}
