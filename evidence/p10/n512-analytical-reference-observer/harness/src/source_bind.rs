//! Evidence binding of every published output to the compiled bytes.
//!
//! The observer's decoding and manifest model are not copies: `main.rs`
//! includes the reviewed `decode.rs` and `model.rs` verbatim from the
//! snapshot-comparison-adapter harness through `#[path]` module attributes, and
//! the crate additionally path-depends on the `nsbu-benchmarks` and
//! `nsbu-solver` sources. The exact included bytes are embedded into the binary
//! at compile time with `include_bytes!`, so every published output reports the
//! SHA-256 of the bytes it was actually compiled from, never a runtime re-read.
//! The sealed `source-inventory.json` beside the harness is embedded the same
//! way: outputs carry its digest, the pinned per-file inventories of both
//! path-dependency crates, and each crate's reproducible aggregate digest
//! (SHA-256 over the sorted `repository_path  sha256\n` lines, recomputed here
//! from the compiled inventory and cross-checked against the sealed value).
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

/// The verbatim `#[path]`-included sources: role, repository-relative path (as
/// recorded in the evidence inventory), the harness-relative path the `#[path]`
/// attribute resolves, and the exact bytes compiled in.
pub(crate) const INCLUDED_SOURCES: [(&str, &str, &str, &[u8]); 2] = [
    (
        "decoder",
        "evidence/p10/snapshot-comparison-adapter/harness/src/decode.rs",
        "../../snapshot-comparison-adapter/harness/src/decode.rs",
        include_bytes!("../../../snapshot-comparison-adapter/harness/src/decode.rs"),
    ),
    (
        "model",
        "evidence/p10/snapshot-comparison-adapter/harness/src/model.rs",
        "../../snapshot-comparison-adapter/harness/src/model.rs",
        include_bytes!("../../../snapshot-comparison-adapter/harness/src/model.rs"),
    ),
];

/// The two path-dependency source crates recorded in the sealed inventory.
pub(crate) const PATH_DEPENDENCY_CRATES: [&str; 2] = ["nsbu-benchmarks", "nsbu-solver"];

/// The sealed evidence inventory, embedded at compile time beside the harness.
pub(crate) const SEALED_INVENTORY: &[u8] = include_bytes!("../../source-inventory.json");

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg_attr(not(test), allow(dead_code))]
fn harness_directory() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// SHA-256 of a harness-relative source file as it sits on disk. Used only by
/// the inventory test to prove the compiled-in bytes match the reviewed files.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn digest_harness_relative(relative: &str) -> Result<String, String> {
    let bytes = crate::provenance::read_bounded_bytes(
        &harness_directory().join(relative),
        crate::provenance::MAX_PLAN_BYTES,
    )?;
    Ok(sha256_hex(&bytes))
}

/// Recompute one path-dependency crate's aggregate digest from the compiled
/// inventory: SHA-256 over `repository_path  sha256\n` lines in string-sorted
/// order, then cross-check against the sealed aggregate.
pub(crate) fn dependency_aggregate(inventory: &Value, crate_name: &str) -> Result<String, String> {
    let dependency = inventory["path_dependencies"]
        .as_array()
        .and_then(|dependencies| {
            dependencies.iter().find(|entry| {
                entry.get("crate").and_then(Value::as_str) == Some(crate_name)
            })
        })
        .ok_or_else(|| format!("compiled inventory carries no {crate_name} dependency"))?;
    let files = dependency["files"]
        .as_array()
        .ok_or_else(|| format!("compiled inventory {crate_name} files are not an array"))?;
    let mut aggregate = Sha256::new();
    for entry in files {
        let relative = entry["repository_path"]
            .as_str()
            .ok_or("compiled inventory file entry carries no repository_path")?;
        let digest = entry["sha256"]
            .as_str()
            .ok_or("compiled inventory file entry carries no sha256")?;
        aggregate.update(format!("{relative}  {digest}\n").as_bytes());
    }
    let computed = format!("{:x}", aggregate.finalize());
    if dependency["aggregate_sha256"].as_str() != Some(computed.as_str()) {
        return Err(format!(
            "compiled inventory {crate_name} aggregate digest does not match its sealed value"
        ));
    }
    Ok(computed)
}

fn sealed_included_digest<'a>(inventory: &'a Value, role: &str) -> Result<&'a str, String> {
    inventory["included_sources"]
        .as_array()
        .and_then(|sources| {
            sources.iter().find(|source| {
                source.get("role").and_then(Value::as_str) == Some(role)
            })
        })
        .and_then(|source| source["sha256"].as_str())
        .ok_or_else(|| format!("compiled inventory carries no sealed digest for role {role:?}"))
}

/// The `external_source_bindings` block carried by every published output:
/// compiled-byte digests for the included sources, both dependency aggregate
/// digests, and the sealed inventory itself — never names or paths alone.
pub(crate) fn included_bindings() -> Result<Value, String> {
    let inventory: Value = serde_json::from_slice(SEALED_INVENTORY)
        .map_err(|error| format!("compiled inventory is invalid JSON: {error}"))?;
    bindings_for(&inventory)
}

/// The binding body against a given inventory shape, so the refusal arms are
/// directly testable (the production call always passes the compiled inventory).
pub(crate) fn bindings_for(inventory: &Value) -> Result<Value, String> {
    let mut sources = Vec::new();
    for (role, repository_path, _harness_relative, compiled_bytes) in INCLUDED_SOURCES {
        let sha256 = sha256_hex(compiled_bytes);
        if sealed_included_digest(inventory, role)? != sha256 {
            return Err(format!(
                "compiled {role} bytes differ from the sealed inventory digest"
            ));
        }
        sources.push(json!({
            "role": role,
            "repository_path": repository_path,
            "included_verbatim_via": "#[path] module attribute",
            "binding": "sha256 of bytes embedded at compile time by include_bytes!",
            "sha256": sha256,
        }));
    }
    let mut aggregates = serde_json::Map::new();
    for crate_name in PATH_DEPENDENCY_CRATES {
        aggregates.insert(crate_name.to_owned(), json!(dependency_aggregate(inventory, crate_name)?));
    }
    Ok(json!({
        "included_decoder_and_model": sources,
        "path_dependency_aggregate_digests": aggregates,
        "aggregate_basis": "SHA-256 over sorted 'repository_path  sha256\\n' lines of the sealed per-file inventories, recomputed from the compiled-in inventory and cross-checked against the sealed aggregates",
        "sealed_inventory": "evidence/p10/n512-analytical-reference-observer/source-inventory.json",
        "sealed_inventory_sha256": sha256_hex(SEALED_INVENTORY),
        "note": "outputs bind the exact compiled decoder/model bytes and both dependency aggregate digests; the sealed inventory pins those digests plus the full path-dependency source inventories",
    }))
}
