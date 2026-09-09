//! Exact dyadic time, independent of a force provider's arithmetic conversion.
use crate::SolverError;

/// Immutable exact clock value. Physical values are tick counts times `2^exponent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickClock {
    exponent: i32,
    target: u128,
    elapsed: u128,
    remaining: u128,
}

impl TickClock {
    /// Restore independently stored elapsed and remaining counts without rounding.
    pub fn restore(
        exponent: i32,
        target: u128,
        elapsed: u128,
        remaining: u128,
    ) -> Result<Self, SolverError> {
        let sum = elapsed
            .checked_add(remaining)
            .ok_or(SolverError::ClockCapacityExceeded)?;
        if remaining == 0 || sum != target {
            return Err(SolverError::InvalidClock);
        }
        Ok(Self {
            exponent,
            target,
            elapsed,
            remaining,
        })
    }

    /// Start at exact zero, a positive number of ticks before the target.
    pub fn from_rest(exponent: i32, target: u128) -> Result<Self, SolverError> {
        Self::restore(exponent, target, 0, target)
    }

    /// Quantum exponent, retained exactly even outside binary64's range.
    pub fn exponent(self) -> i32 {
        self.exponent
    }

    /// Immutable target tick count.
    pub fn target(self) -> u128 {
        self.target
    }

    /// Elapsed ticks; startup evaluators consume this value independently.
    pub fn elapsed(self) -> u128 {
        self.elapsed
    }

    /// Positive remaining ticks; similarity evaluators consume this independently.
    pub fn remaining(self) -> u128 {
        self.remaining
    }

    /// Form start, quarter, half, three-quarter and endpoint clocks directly.
    /// A failed request leaves this clock unchanged and never quantizes an interval.
    pub fn stages(self, ticks: u128) -> Result<[Self; 5], SolverError> {
        if ticks == 0 || !ticks.is_multiple_of(4) {
            return Err(SolverError::InvalidStep);
        }
        if ticks >= self.remaining {
            return Err(SolverError::ClockCapacityExceeded);
        }
        let mut stages = [self; 5];
        for (index, stage) in stages.iter_mut().enumerate().skip(1) {
            let offset = (ticks / 4) * index as u128;
            // The validated interval preserves elapsed + remaining = target.
            *stage = Self {
                exponent: self.exponent,
                target: self.target,
                elapsed: self
                    .elapsed
                    .checked_add(offset)
                    .ok_or(SolverError::ClockCapacityExceeded)?,
                remaining: self
                    .remaining
                    .checked_sub(offset)
                    .ok_or(SolverError::ClockCapacityExceeded)?,
            };
        }
        Ok(stages)
    }
}
