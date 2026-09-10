//! Canonical artifact round trips, bounded refusals and corruption/truncation controls.
mod checkpoint_support;
use checkpoint_support::artifact;
use nsbu_solver::checkpoint::{
    archive::{encoded_len, write, ArtifactArchive},
    artifacts::{Catalog, Kind},
    CheckpointError,
};

fn encoded(count: usize) -> Vec<u8> {
    let entries = [
        Kind::Problem,
        Kind::Execution,
        Kind::Force,
        Kind::ForceCoverage,
        Kind::Policy,
        Kind::Lineage,
        Kind::Reference,
    ]
    .map(artifact);
    let catalog = Catalog::new(&entries[..count], 21).unwrap();
    let required = encoded_len(catalog).unwrap();
    assert_eq!(required, 9 + count * 51);
    let mut bytes = vec![0x55; required + 2];
    assert_eq!(
        write(catalog, &mut bytes[..required - 1]),
        Err(CheckpointError::ResourceLimit)
    );
    assert!(bytes.iter().all(|b| *b == 0x55));
    assert_eq!(write(catalog, &mut bytes), Ok(required));
    assert_eq!(&bytes[required..], &[0x55; 2]);
    bytes.truncate(required);
    bytes
}

#[test]
fn both_catalog_shapes_preserve_version_lengths_hashes_and_exact_content() {
    for count in [6, 7] {
        let bytes = encoded(count);
        assert_eq!(&bytes[..8], b"NSBUAR01");
        assert_eq!(bytes[8], count as u8);
        assert_eq!(&bytes[41..57], &3u128.to_le_bytes());
        let archive = ArtifactArchive::read(&bytes, bytes.len()).unwrap();
        assert_eq!(archive.entries().len(), count);
        for entry in archive.entries() {
            assert_eq!(entry.identity().bytes(), checkpoint_support::ABC);
            assert_eq!(entry.bytes(), b"abc");
        }
        let catalog = Catalog::new(archive.entries(), 21).unwrap();
        assert_eq!(catalog.get(Kind::Reference).is_some(), count == 7);
        let mut again = vec![0; bytes.len()];
        write(catalog, &mut again).unwrap();
        assert_eq!(again, bytes);
        assert_eq!(
            ArtifactArchive::read(&bytes, bytes.len() - 1).unwrap_err(),
            CheckpointError::ResourceLimit
        );
    }
}

#[test]
fn incomplete_excessive_and_modified_archives_never_publish_a_catalog() {
    let bytes = encoded(7);
    for stop in 0..bytes.len() {
        assert!(ArtifactArchive::read(&bytes[..stop], bytes.len()).is_err());
    }
    for at in 0..8 {
        let mut bad = bytes.clone();
        bad[at] ^= 1;
        assert_eq!(
            ArtifactArchive::read(&bad, bad.len()).unwrap_err(),
            CheckpointError::InvalidEncoding
        );
    }
    for count in [0, 5, 8, 255] {
        let mut bad = bytes.clone();
        bad[8] = count;
        assert_eq!(
            ArtifactArchive::read(&bad, bad.len()).unwrap_err(),
            CheckpointError::InvalidCatalog
        );
    }
    for at in [9, 57, bytes.len() - 1] {
        let mut bad = bytes.clone();
        bad[at] ^= 1;
        assert_eq!(
            ArtifactArchive::read(&bad, bad.len()).unwrap_err(),
            CheckpointError::HashMismatch
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        ArtifactArchive::read(&trailing, trailing.len()).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
}

#[test]
fn malformed_identity_and_declared_lengths_fail_before_content_access() {
    let original = encoded(6);
    let mut bad = original.clone();
    bad[9..41].fill(0);
    assert_eq!(
        ArtifactArchive::read(&bad, bad.len()).unwrap_err(),
        CheckpointError::InvalidEncoding
    );
    let mut overflow = original.clone();
    overflow[41..57].copy_from_slice(&u128::MAX.to_le_bytes());
    assert_eq!(
        ArtifactArchive::read(&overflow, overflow.len()).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    let mut empty = original;
    empty[41..57].fill(0);
    assert_eq!(
        ArtifactArchive::read(&empty, empty.len()).unwrap_err(),
        CheckpointError::MissingArtifact
    );
}
