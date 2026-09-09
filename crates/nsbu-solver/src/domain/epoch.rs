//! Checked numerical/plan generation identifiers.
use crate::SolverError;

/// A caller-assigned generation identifier; wrapping is prohibited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch(pub u128);

impl Epoch {
    /// Compute a new generation without changing the old one on failure.
    pub fn next(self) -> Result<Self, SolverError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(SolverError::EpochExhausted)
    }
}
