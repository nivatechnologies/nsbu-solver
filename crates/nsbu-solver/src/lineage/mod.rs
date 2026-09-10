//! Bounded provenance declarations, separate from numerical qualification and checkpoint trust.
mod image;
mod record;
mod registry;
pub use image::PhysicalImage;
pub use record::{Digest, Origin, Profile, Record, RecordId};
pub use registry::Registry;

/// Refusal of a provenance declaration; no physical state is modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineageError {
    /// The all-zero digest is reserved for missing identity.
    MissingIdentity,
    /// Storage, visit or attempt allowance is insufficient.
    CapacityExceeded,
    /// Caller storage must be empty before creating a registry.
    OccupiedStorage,
    /// The parent does not belong to this registry or has not been recorded.
    UnknownParent,
    /// Invalidated history cannot produce a new continuation.
    InvalidatedParent,
    /// A comparison or continuation changes the mathematical problem.
    DifferentProblem,
    /// A continuation changes its numerical profile without an explicit transfer.
    DifferentProfile,
    /// Exact times do not describe the declared origin or continuation.
    InvalidClock,
}
