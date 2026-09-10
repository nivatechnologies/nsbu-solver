//! Bounded checkpoint artifacts; byte integrity does not establish numerical qualification.
pub mod archive;
pub mod artifacts;
mod bytes;
mod failure;
pub mod history;
pub mod physical;
mod record;

/// Checkpoint content admission failed before publishing a restored physical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointError {
    /// Empty required content is not an artifact.
    MissingArtifact,
    /// The declared byte allowance or checked size was exceeded.
    ResourceLimit,
    /// Supplied content does not match its expected SHA-256 identity.
    HashMismatch,
    /// Required entries are missing, duplicated, excessive or out of canonical order.
    InvalidCatalog,
    /// Truncated, excessive, malformed or unsupported versioned encoding.
    InvalidEncoding,
    /// Decoded log or exact clock violates the solver's history invariants.
    InvalidHistory(crate::SolverError),
}
