//! Bounded empirical nominal core/annulus coverage attached to accepted v2 reports.
mod plan;
mod report;
mod workspace;

pub use plan::{CoverageFamilyBounds, CoverageFamilyPlan, CoverageFamilyWork};
pub use report::{CoverageFamilySample, CoverageSamplingMetadata};
pub use workspace::{CoverageFamilyError, CoverageFamilyWorkspace};
