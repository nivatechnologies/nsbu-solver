#![allow(clippy::too_many_lines)]
//! Sealed source-inventory verification: the harness includes the reviewed
//! decoder and model verbatim through `#[path]` attributes and path-depends on
//! the `nsbu-benchmarks` and `nsbu-solver` crates. This test re-hashes every
//! file pinned in `source-inventory.json` beside the harness, checks the
//! runtime bindings every published output carries, and proves the pinned
//! inventories are complete against the files actually on disk.
use crate::fixtures;
use crate::source_bind;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("repository root")
}

fn inventory() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../source-inventory.json")
        .canonicalize()
        .expect("sealed source inventory beside the harness");
    serde_json::from_slice(&fs::read(&path).expect("inventory bytes")).expect("inventory json")
}

fn sha256_file(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).expect("file bytes")))
}

/// Count non-lock files under a directory recursively, mirroring the
/// generator's rule so an unlisted extra file cannot hide.
fn disk_file_count(root: &Path) -> usize {
    let mut stack = vec![root.to_path_buf()];
    let mut count = 0;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("directory listing") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_none_or(|n| n != "Cargo.lock") {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn inventory_seals_the_included_decoder_and_model_bytes() {
    let inventory = inventory();
    assert_eq!(
        inventory["schema"],
        "p10-n512-analytical-reference-observer-source-inventory-v1"
    );
    let sources = inventory["included_sources"].as_array().expect("included sources");
    assert_eq!(sources.len(), source_bind::INCLUDED_SOURCES.len());
    for entry in sources {
        let role = entry["role"].as_str().expect("role");
        let repository_path = entry["repository_path"].as_str().expect("repository path");
        let recorded = entry["sha256"].as_str().expect("digest");
        assert_eq!(
            sha256_file(&repo_root().join(repository_path)),
            recorded,
            "{role}: bytes on disk differ from the sealed digest"
        );
        let (_, inventory_path, harness_relative, compiled_bytes) = source_bind::INCLUDED_SOURCES
            .iter()
            .find(|(name, _, _, _)| *name == role)
            .expect("runtime binding for role");
        assert_eq!(
            source_bind::sha256_hex(compiled_bytes),
            recorded,
            "{role}: the compiled-in bytes differ from the sealed digest"
        );
        assert_eq!(*inventory_path, repository_path);
        assert_eq!(
            source_bind::digest_harness_relative(harness_relative).expect("runtime digest"),
            recorded,
            "{role}: the #[path]-included bytes differ from the sealed digest"
        );
    }
}

#[test]
fn published_outputs_carry_the_sealed_included_digests() {
    let inventory = inventory();
    let record_path = fixtures::write_clock_record(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/analytical-reference-inventory"),
        "inventory-output",
    );
    let output = crate::n512::n512_ledger(
        &record_path,
        crate::Request {
            velocity_samples: 512,
            pressure_samples: 1024,
            force_samples: 1024,
            workers: 32,
            root_budget: 32,
            max_reference_evaluations: 1 << 40,
            cap: 1 << 48,
            velocity_floor: 1.0,
            pressure_floor: 1.0,
            backend: "rustfft-6.4.1-avx-avx2-fma".to_owned(),
        },
        None,
    )
    .expect("ledger output");
    let value: Value = serde_json::from_str(&output).expect("json");
    let published = value["external_source_bindings"]["included_decoder_and_model"]
        .as_array()
        .expect("published included bindings");
    assert_eq!(published.len(), 2);
    for entry in published {
        let role = entry["role"].as_str().expect("role");
        let sealed = inventory["included_sources"]
            .as_array()
            .expect("included sources")
            .iter()
            .find(|source| source["role"].as_str() == Some(role))
            .expect("sealed entry for role");
        assert_eq!(entry["sha256"], sealed["sha256"], "{role}: output digest drifted");
        assert!(
            entry["binding"]
                .as_str()
                .is_some_and(|basis| basis.contains("compile time")),
            "{role}: outputs must bind compiled bytes, not runtime reads"
        );
    }
    let aggregates = value["external_source_bindings"]["path_dependency_aggregate_digests"]
        .as_object()
        .expect("published aggregates");
    for dependency in inventory["path_dependencies"].as_array().expect("dependencies") {
        let name = dependency["crate"].as_str().expect("crate");
        assert_eq!(
            aggregates[name], dependency["aggregate_sha256"],
            "{name}: published aggregate differs from the sealed aggregate"
        );
    }
    let sealed_bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../source-inventory.json"),
    )
    .expect("inventory bytes");
    assert_eq!(
        value["external_source_bindings"]["sealed_inventory_sha256"],
        format!("{:x}", Sha256::digest(&sealed_bytes)),
        "published inventory digest differs from the sealed file"
    );
}

#[test]
fn stale_decoder_inventory_refuses_and_current_head_bytes_bind() {
    // The decoder drifted from the a7f7 base bytes (6da6f8ac...) to the
    // current 30dc45f bytes (ac11a172...). An inventory still sealing the
    // stale digest must be refused against the compiled bytes, and the
    // rebased inventory must bind with the current bytes on disk.
    const STALE_A7F7_DECODER: &str = "6da6f8ac65390d09fd2a04ae04cea4968048b1125247bb3c815a861aa6716d9d";
    const CURRENT_30DC45F_DECODER: &str = "ac11a172ac38cb92c7665ceecb3ee6892d437afd99a8ff3246a61a38e9e5074c";
    let inventory = inventory();
    let decoder = &inventory["included_sources"][0];
    assert_eq!(decoder["role"], "decoder");
    assert_eq!(
        decoder["sha256"].as_str().expect("digest"),
        CURRENT_30DC45F_DECODER,
        "the sealed inventory must pin the current 30dc45f decoder bytes"
    );
    assert_eq!(
        sha256_file(&repo_root().join(decoder["repository_path"].as_str().expect("path"))),
        CURRENT_30DC45F_DECODER,
        "decoder bytes on disk differ from 30dc45f"
    );
    let compiled_digest = source_bind::sha256_hex(source_bind::INCLUDED_SOURCES[0].3);
    assert_eq!(compiled_digest, CURRENT_30DC45F_DECODER, "compiled bytes differ from 30dc45f");

    let mut stale = inventory.clone();
    stale["included_sources"][0]["sha256"] = Value::from(STALE_A7F7_DECODER);
    let error = source_bind::bindings_for(&stale)
        .expect_err("an a7f7-era decoder digest must refuse the current compiled bytes");
    assert!(
        error.contains("compiled decoder bytes differ from the sealed inventory digest"),
        "{error}"
    );

    let bound = source_bind::bindings_for(&inventory).expect("the rebased inventory must bind");
    assert_eq!(
        bound["included_decoder_and_model"][0]["sha256"],
        CURRENT_30DC45F_DECODER,
        "published decoder binding drifted from the current bytes"
    );
}

#[test]
fn inventory_seals_both_path_dependency_crates_completely() {
    let inventory = inventory();
    let dependencies = inventory["path_dependencies"].as_array().expect("path dependencies");
    assert_eq!(dependencies.len(), source_bind::PATH_DEPENDENCY_CRATES.len());
    for dependency in dependencies {
        let name = dependency["crate"].as_str().expect("crate name");
        assert!(source_bind::PATH_DEPENDENCY_CRATES.contains(&name));
        let files = dependency["files"].as_array().expect("file list");
        let crate_root = repo_root().join("crates").join(name);
        assert_eq!(
            disk_file_count(&crate_root),
            files.len(),
            "{name}: files on disk are not all pinned in the inventory"
        );
        let mut aggregate = Sha256::new();
        let mut previous = String::new();
        for entry in files {
            let relative = entry["repository_path"].as_str().expect("repository path");
            assert!(relative > previous.as_str(), "{name}: inventory paths must be string-sorted and unique");
            previous = relative.to_owned();
            assert_eq!(
                sha256_file(&repo_root().join(relative)),
                entry["sha256"].as_str().expect("digest"),
                "{relative}: bytes on disk differ from the sealed digest"
            );
            aggregate.update(format!("{relative}  {}\n", entry["sha256"].as_str().expect("digest")).as_bytes());
        }
        assert_eq!(
            format!("{:x}", aggregate.finalize()),
            dependency["aggregate_sha256"].as_str().expect("aggregate"),
            "{name}: aggregate digest mismatch"
        );
    }
}
