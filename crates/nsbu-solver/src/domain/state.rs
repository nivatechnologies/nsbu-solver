//! Independently owned payloads; numerical mutation is reserved for transactional attempts.
use super::{Epoch, ResourcePlan, TickClock};
use crate::storage::filled;
use crate::{Complex64, SolverError};

/// An independently allocated from-rest physical state.
/// No reference evaluator or shared mutable storage is accepted by this constructor.
#[derive(Debug)]
pub struct SpectralState {
    plan: ResourcePlan,
    clock: TickClock,
    epoch: Epoch,
    components: [Vec<Complex64>; 3],
}

impl SpectralState {
    /// Allocate zero velocity only after the complete declared reservation is approved.
    pub fn from_rest(
        plan: ResourcePlan,
        clock: TickClock,
        epoch: Epoch,
    ) -> Result<Self, SolverError> {
        if clock.elapsed() != 0 {
            return Err(SolverError::InvalidClock);
        }
        let n = plan.domain().layout().half_len();
        Ok(Self {
            plan,
            clock,
            epoch,
            components: [
                filled(n, Complex64::new(0.0, 0.0))?,
                filled(n, Complex64::new(0.0, 0.0))?,
                filled(n, Complex64::new(0.0, 0.0))?,
            ],
        })
    }

    /// Immutable plan associated with this storage.
    pub fn plan(&self) -> ResourcePlan {
        self.plan
    }

    /// Exact physical time, independently owned by this trajectory.
    pub fn clock(&self) -> TickClock {
        self.clock
    }

    /// Numerical generation, separate from the plan's immutable mathematical data.
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Read-only normalized Fourier component; invalid axes are refused.
    pub fn component(&self, axis: usize) -> Result<&[Complex64], SolverError> {
        self.components
            .get(axis)
            .map(Vec::as_slice)
            .ok_or(SolverError::InvalidIndex)
    }
}
