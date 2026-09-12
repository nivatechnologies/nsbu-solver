//! Sampled physical force-grid differences from three unchanged family states.
mod plan;
mod report;
mod workspace;

use nsbu_solver::SolverError;
pub use plan::{ForcePhysicalBounds, ForcePhysicalPlan, ForcePhysicalWork};
pub use report::{
    ForcePhysicalQuantity, ForcePhysicalSample, ForcePhysicalStatus, FORCE_PHYSICAL_QUANTITIES,
};
pub use workspace::ForcePhysicalWorkspace;

/// Terminal measurement failures. A failed request never publishes a partial report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForcePhysicalError {
    /// Family, raw sample, clock, identity, or profile did not match admission.
    InvalidFamily,
    /// Sampling or a checked resource operation failed.
    Numerical(SolverError),
    /// A prior charged attempt failed permanently.
    Terminated,
}

impl From<SolverError> for ForcePhysicalError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}
