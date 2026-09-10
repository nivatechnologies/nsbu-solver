//! Empirical evidence review, separate from integration and from certified error enclosures.
pub mod budget;
pub mod observation;
pub mod policy;
pub mod reconstruction;
pub mod refinement;
pub mod review;
pub mod times;

/// A verification input cannot be admitted under the declared finite profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationError {
    /// A tolerance, measurement or policy parameter is nonfinite or outside its domain.
    InvalidValue,
    /// A documented floor has no supporting analysis identifier.
    MissingAnalysis,
    /// A tested-time manifest is empty, unordered, duplicated or uses inconsistent exact clocks.
    InvalidTimes,
    /// The supplied records exceed the caller's declared finite review allowance.
    CapacityExceeded,
    /// Reconstruction clocks or their finite arithmetic violate the Hermite contract.
    ReconstructionGeometry(crate::SolverError),
    /// A diagnostic probe is on an integrator stage or its quarter ticks do not exist.
    InvalidProbe,
    /// A mandatory convergence channel was weakened to a sensitivity-only comparison.
    InvalidRequirement,
    /// The conservatively accumulated channel allocations exceed the total tolerance.
    ExcessAllocation,
    /// No required observables were declared.
    MissingPolicy,
    /// The same observable key appears more than once in the frozen policy.
    DuplicateObservable,
    /// A record is out of order, duplicated, or belongs to a different exact time/observable.
    UnexpectedObservation,
    /// No off-stage reconstruction comparison was supplied.
    MissingReconstruction,
    /// Primary evidence was cropped to common modes or aligned before comparison.
    InvalidComparisonScope,
}
