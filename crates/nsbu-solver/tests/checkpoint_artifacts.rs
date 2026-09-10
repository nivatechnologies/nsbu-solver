//! Known-answer integrity, exact input preservation and bounded complete catalog tests.
use nsbu_solver::checkpoint::{
    artifacts::{Artifact, Catalog, Kind},
    CheckpointError,
};
use nsbu_solver::lineage::Digest;

mod checkpoint_support;
use checkpoint_support::{artifact, ABC};

#[test]
fn hashing_preserves_exact_bytes_and_refuses_corruption_before_catalog_admission() {
    let input = artifact(Kind::Problem);
    assert_eq!(input.kind(), Kind::Problem);
    assert_eq!(input.identity().bytes(), ABC);
    assert_eq!(input.bytes(), b"abc");
    let identity = input.identity();
    for changed in [b"abd".as_slice(), b"abc\n", b"abc\r\n"] {
        assert_eq!(
            Artifact::verify(Kind::Problem, identity, changed, 6).unwrap_err(),
            CheckpointError::HashMismatch
        );
    }
    assert_eq!(
        Artifact::verify(Kind::Force, identity, b"abc", 2).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    assert_eq!(
        Artifact::verify(Kind::Force, identity, b"", 0).unwrap_err(),
        CheckpointError::MissingArtifact
    );
    assert_eq!(
        Artifact::verify(Kind::Force, Digest::new([1; 32]).unwrap(), b"abc", 3).unwrap_err(),
        CheckpointError::HashMismatch
    );
}

#[test]
fn canonical_catalog_is_complete_bounded_and_keeps_optional_reference_explicit() {
    let kinds = [
        Kind::Problem,
        Kind::Execution,
        Kind::Force,
        Kind::ForceCoverage,
        Kind::Policy,
        Kind::Lineage,
        Kind::Reference,
    ];
    let entries = kinds.map(artifact);
    for count in [6, 7] {
        let catalog = Catalog::new(&entries[..count], count * 3).unwrap();
        assert_eq!(catalog.total_bytes(), count * 3);
        assert_eq!(catalog.entries().len(), count);
        for kind in kinds.into_iter().take(count) {
            let present = catalog.get(kind).unwrap();
            assert_eq!(present.kind(), kind);
            assert_eq!(present.bytes(), b"abc");
        }
        assert_eq!(catalog.get(Kind::Reference).is_some(), count == 7);
        assert_eq!(
            Catalog::new(&entries[..count], count * 3 - 1).unwrap_err(),
            CheckpointError::ResourceLimit
        );
    }
    assert_eq!(
        Catalog::new(&entries, usize::MAX).unwrap().total_bytes(),
        21
    );
    assert_eq!(
        Catalog::new(&entries, 0).unwrap_err(),
        CheckpointError::ResourceLimit
    );
    for count in 0..6 {
        assert_eq!(
            Catalog::new(&entries[..count], 21).unwrap_err(),
            CheckpointError::InvalidCatalog
        );
    }
    let excessive = [artifact(Kind::Problem); 8];
    assert_eq!(
        Catalog::new(&excessive, 24).unwrap_err(),
        CheckpointError::InvalidCatalog
    );
    for index in 0..7 {
        let mut changed = entries;
        changed[index] = artifact(kinds[(index + 1) % 7]);
        assert_eq!(
            Catalog::new(&changed, 21).unwrap_err(),
            CheckpointError::InvalidCatalog
        );
    }
}
