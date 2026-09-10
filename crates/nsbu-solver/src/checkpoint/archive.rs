//! Versioned artifact-only container. It contains no physical state or accepted-window label.
use super::bytes::{put, Cursor};
use super::{
    artifacts::{Artifact, Catalog, Kind, ORDER},
    CheckpointError,
};
use crate::lineage::Digest;
const MAGIC: &[u8; 8] = b"NSBUAR01";

/// Validated borrowed artifact bytes with bounded parsing and no heap allocation.
/// Hash integrity and canonical slots do not establish the contents' semantic truth.
#[derive(Debug)]
pub struct ArtifactArchive<'a> {
    entries: [Artifact<'a>; 7],
    count: usize,
}
impl<'a> ArtifactArchive<'a> {
    /// Bound the complete input before parsing/hashing; reject trailing bytes and bad digests.
    pub fn read(bytes: &'a [u8], maximum_bytes: usize) -> Result<Self, CheckpointError> {
        if bytes.len() > maximum_bytes {
            return Err(CheckpointError::ResourceLimit);
        }
        let mut cursor = Cursor { remaining: bytes };
        if cursor.take(8)? != MAGIC {
            return Err(CheckpointError::InvalidEncoding);
        }
        let count = usize::from(cursor.take(1)?[0]);
        if !(6..=7).contains(&count) {
            return Err(CheckpointError::InvalidCatalog);
        }
        let first = cursor.artifact(Kind::Problem)?;
        let mut entries = [first; 7];
        for (entry, kind) in entries[1..count].iter_mut().zip(ORDER.into_iter().skip(1)) {
            *entry = cursor.artifact(kind)?;
        }
        if !cursor.remaining.is_empty() {
            return Err(CheckpointError::InvalidEncoding);
        }
        Ok(Self { entries, count })
    }
    /// Complete preserved catalog in canonical order, with optional reference last.
    pub fn entries(&self) -> &[Artifact<'a>] {
        &self.entries[..self.count]
    }
}

/// Exact encoding size, checked before touching the caller's output storage.
pub fn encoded_len(catalog: Catalog<'_>) -> Result<usize, CheckpointError> {
    catalog.entries().iter().try_fold(9usize, |size, entry| {
        size.checked_add(48)
            .and_then(|size| size.checked_add(entry.bytes().len()))
            .ok_or(CheckpointError::ResourceLimit)
    })
}

/// Write the admitted catalog into caller storage. A short output is unchanged on refusal.
/// Each entry is its SHA-256 identity, little-endian u128 byte length, then exact content.
pub fn write(catalog: Catalog<'_>, output: &mut [u8]) -> Result<usize, CheckpointError> {
    let required = encoded_len(catalog)?;
    if output.len() < required {
        return Err(CheckpointError::ResourceLimit);
    }
    let mut position = 0;
    put(output, &mut position, MAGIC);
    put(output, &mut position, &[catalog.entries().len() as u8]);
    for entry in catalog.entries() {
        put(output, &mut position, &entry.identity().bytes());
        put(
            output,
            &mut position,
            &(entry.bytes().len() as u128).to_le_bytes(),
        );
        put(output, &mut position, entry.bytes());
    }
    Ok(position)
}
impl<'a> Cursor<'a> {
    fn artifact(&mut self, kind: Kind) -> Result<Artifact<'a>, CheckpointError> {
        let hash = self.array()?;
        let identity = Digest::new(hash).map_err(|_| CheckpointError::InvalidEncoding)?;
        let size = self.array()?;
        let count = usize::try_from(u128::from_le_bytes(size))
            .map_err(|_| CheckpointError::ResourceLimit)?;
        let bytes = self.take(count)?;
        Artifact::verify(kind, identity, bytes, count)
    }
}
