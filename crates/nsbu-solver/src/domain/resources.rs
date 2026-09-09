//! Checked allocation ledger. Planning performs no numerical-grid allocations.
use super::{Domain, Epoch};
use crate::SolverError;

/// Mandatory caller declarations beyond the reviewed base arrays.
/// Actual FFT/provider planners must supply their complete reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtraStorage {
    /// FFT plans and scratch, including any retained library-owned storage.
    pub fft: usize,
    /// Force evaluator workspaces and immutable tables.
    pub force: usize,
    /// Diagnostics, history and output staging.
    pub diagnostics: usize,
    /// Metadata and an explicit allocator-overhead allowance.
    pub overhead: usize,
}

/// Immutable reservation approved against a caller-supplied byte cap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourcePlan {
    domain: Domain,
    epoch: Epoch,
    classes: [usize; 8],
    total: usize,
}

impl ResourcePlan {
    /// Check every class and the full sum before allocating any solver payload.
    /// This ledger validates declarations; it cannot discover undeclared provider memory.
    pub fn new(
        domain: Domain,
        extra: ExtraStorage,
        cap: usize,
        epoch: Epoch,
    ) -> Result<Self, SolverError> {
        let retained = domain.layout().half_len();
        let padded = domain.padded_layout()?;
        let classes = [
            bytes(retained, 12 * 48)?,
            bytes(padded.real_len(), 3 * 24)?,
            bytes(padded.half_len(), 48)?,
            bytes(retained, 6 * 8)?,
            extra.fft,
            extra.force,
            extra.diagnostics,
            extra.overhead,
        ];
        let total = classes.iter().try_fold(0_usize, |sum, value| {
            sum.checked_add(*value).ok_or(SolverError::SizeOverflow)
        })?;
        if total > cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            domain,
            epoch,
            classes,
            total,
        })
    }

    /// Caller-assigned immutable planning generation, independent of state generations.
    pub fn epoch(self) -> Epoch {
        self.epoch
    }

    /// The exact physical domain to which this reservation applies.
    pub fn domain(self) -> Domain {
        self.domain
    }

    /// Retained vectors, padded reals, padded complex, tables, FFT, force, diagnostics, overhead.
    pub fn classes(self) -> [usize; 8] {
        self.classes
    }

    /// Total declared bytes. This is a planning reservation, not measured resident memory.
    pub fn total(self) -> usize {
        self.total
    }
}

fn bytes(count: usize, width: usize) -> Result<usize, SolverError> {
    count.checked_mul(width).ok_or(SolverError::SizeOverflow)
}
