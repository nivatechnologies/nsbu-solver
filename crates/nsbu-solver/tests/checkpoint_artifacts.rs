//! Known-answer integrity, exact input preservation and bounded complete catalog tests.
use nsbu_solver::checkpoint::{
    artifacts::{Artifact, Catalog, Kind},
    CheckpointError,
};
use nsbu_solver::lineage::Digest;

// Published SHA-256 known-answer vector for the three ASCII bytes "abc".
const ABC: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];
fn artifact(kind: Kind) -> Artifact<'static> {
    Artifact::verify(kind, Digest::new(ABC).unwrap(), b"abc", 3).unwrap()
}

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
