//! Bounded immutable exact-v2 original-force tables for one explicit clock slab.
//!
//! This standalone integration-scale owner does not alter [`crate::v2_run::Run`]
//! or [`super::V2Family`]. Observer force on a doubled grid requires a separate
//! table plan and identity.
mod adapter;
mod identity;
mod plan;
mod table;

pub use adapter::{
    SharedForceAdapter, SharedForceAdapterBounds, SharedForceAdapterSet, SharedForceAdapterSetPlan,
    SharedForceAdapterWork, SharedForceAttempt, SharedForceStream,
};
pub use plan::{
    SharedForceBinding, SharedForceBounds, SharedForceClock, SharedForceTablePlan, SharedForceWork,
};
pub use table::{SharedForceCopy, SharedForceError, SharedForceTable};
