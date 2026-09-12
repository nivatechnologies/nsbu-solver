//! Frozen exact-v2 review-profile admission without fabricated measurements or budgets.
mod admission;
mod geometry;
mod inventory;

pub use admission::{
    AdmittedProfile, ProfileBounds, ProfileCaps, ProfileError, ProfileInputs, ProfileStatus,
    RequiredRow,
};
pub use geometry::ReviewGeometry;
pub use inventory::{
    semantics_identity, MandatoryObservableGap, ObservableDescriptor, ObservableQuantity,
    ObservableRegion, ObservableSource, ObservableStatistic, ObservableUnits, FIRST_KEY,
    MANDATORY_GAPS, OBSERVABLES, OBSERVABLE_COUNT,
};

/// Raw 32-byte mathematical case digest represented by [`crate::CASE_SHA256`].
pub const PROBLEM_IDENTITY: [u8; 32] = [
    0xe1, 0x23, 0x6f, 0x7b, 0x3c, 0x51, 0x53, 0x7a, 0xcd, 0x17, 0x38, 0x14, 0x02, 0xca, 0x42, 0x0b,
    0xa7, 0x87, 0x2a, 0x7b, 0x9d, 0xbc, 0x64, 0xb2, 0xf0, 0xd9, 0xd5, 0x10, 0x8a, 0x46, 0x8f, 0x7e,
];
/// Version tag, count and nine bytes per canonical descriptor.
pub const SEMANTICS_BYTES: usize =
    b"NSBUV2OBSERVABLES0001".len() + 16 + OBSERVABLE_COUNT * 9 + 16 + MANDATORY_GAPS.len();
