//! Prescribed-force contracts carry exact clocks, explicit storage and bounded work reports.
use crate::{domain::TickClock, Complex64, SolverError};

/// Immutable cost declaration for an admitted force implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceLimits {
    /// Complete owned provider storage, including fixed caches and tables.
    pub storage_bytes: usize,
    /// Maximum abstract provider work units in one evaluation.
    pub work_units: usize,
    /// Maximum scalar 3D transforms performed by one force evaluation.
    pub scalar_transforms: usize,
    /// Require attempted ticks <= remaining/divisor; use 20 for the initial MMS profile.
    pub remaining_divisor: u128,
}

/// Actual consumption declared by a completed force evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ForceWork {
    /// Consumed provider work units, including failed internal iterations.
    pub work_units: usize,
    /// Scalar 3D transforms consumed by this call.
    pub scalar_transforms: usize,
}

/// A prescribed source, independent of evolving velocity and analytical reference access.
/// Implementations must enforce their declared limits internally, including error paths.
/// Unknown costs return None and cannot enter the bounded spectral RHS profile.
pub trait PrescribedForce {
    /// Complete immutable implementation-specific bounds; not an inferred cost estimate.
    fn limits(&self) -> Option<ForceLimits>;
    /// Fill normalized unprojected retained coefficients without allocating or resetting state.
    fn evaluate(
        &mut self,
        time: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError>;
}
