//! Content-authenticated borrowed inputs, with fixed catalog traversal and bounded hash input.
use super::CheckpointError;
use crate::lineage::Digest;
use sha2::{Digest as HashDigest, Sha256};

/// Canonical artifact slots. The reference definition is optional for a diagnostic-only run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Kind {
    /// Complete mathematical input, including physical parameters and initial data.
    Problem,
    /// Grid, arithmetic, compiler/runtime, layout/planner versions and execution settings.
    Execution,
    /// Prescribed-force implementation and immutable mathematical parameters.
    Force,
    /// Full evaluator coverage declaration and supporting content.
    ForceCoverage,
    /// Frozen observable inventory, tolerances and diagnostic scope.
    Policy,
    /// Full ancestry and inherited-error report content; semantics require separate review.
    Lineage,
    /// Independent analytical/reference implementation and definition, when used.
    Reference,
}
pub(super) const ORDER: [Kind; 7] = [
    Kind::Problem,
    Kind::Execution,
    Kind::Force,
    Kind::ForceCoverage,
    Kind::Policy,
    Kind::Lineage,
    Kind::Reference,
];

/// Verified content bytes. Verification checks integrity, not their semantic adequacy or truth.
#[derive(Debug, Clone, Copy)]
pub struct Artifact<'a> {
    kind: Kind,
    identity: Digest,
    bytes: &'a [u8],
}
impl<'a> Artifact<'a> {
    /// Bound input before hashing, require nonempty content, and compare the expected identity.
    /// The pinned software SHA-256 implementation owns no input-sized scratch allocation.
    pub fn verify(
        kind: Kind,
        identity: Digest,
        bytes: &'a [u8],
        maximum_bytes: usize,
    ) -> Result<Self, CheckpointError> {
        if bytes.len() > maximum_bytes {
            return Err(CheckpointError::ResourceLimit);
        }
        if bytes.is_empty() {
            return Err(CheckpointError::MissingArtifact);
        }
        let computed: [u8; 32] = Sha256::digest(bytes).into();
        if computed != identity.bytes() {
            return Err(CheckpointError::HashMismatch);
        }
        Ok(Self {
            kind,
            identity,
            bytes,
        })
    }
    /// Canonical semantic slot; this does not parse or validate that slot's schema.
    pub fn kind(self) -> Kind {
        self.kind
    }
    /// SHA-256 identity verified against the complete borrowed bytes.
    pub fn identity(self) -> Digest {
        self.identity
    }
    /// Exact input, including every byte and original line ending.
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}

/// Immutable six-required/one-optional catalog with a checked total input allowance.
#[derive(Debug, Clone, Copy)]
pub struct Catalog<'a> {
    entries: &'a [Artifact<'a>],
    total_bytes: usize,
}
impl<'a> Catalog<'a> {
    /// Require every mandatory slot exactly once in canonical order before exposing a catalog.
    pub fn new(entries: &'a [Artifact<'a>], maximum_bytes: usize) -> Result<Self, CheckpointError> {
        if !(6..=7).contains(&entries.len()) {
            return Err(CheckpointError::InvalidCatalog);
        }
        let mut remaining = maximum_bytes;
        for (entry, required) in entries.iter().zip(ORDER) {
            if entry.kind != required {
                return Err(CheckpointError::InvalidCatalog);
            }
            remaining = remaining
                .checked_sub(entry.bytes.len())
                .ok_or(CheckpointError::ResourceLimit)?;
        }
        Ok(Self {
            entries,
            total_bytes: maximum_bytes - remaining,
        })
    }
    /// Present artifact, with absence possible only for the optional reference slot.
    pub fn get(self, kind: Kind) -> Option<Artifact<'a>> {
        self.entries.get(kind as usize).copied()
    }
    /// Exact aggregate byte count admitted by this catalog.
    pub fn total_bytes(self) -> usize {
        self.total_bytes
    }
    /// Borrow all entries in their preserved canonical order.
    pub fn entries(self) -> &'a [Artifact<'a>] {
        self.entries
    }
}
