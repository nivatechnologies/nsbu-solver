//! Exact tested-time manifests. A refined sample set is not a continuous-time enclosure.
use super::VerificationError;
use crate::domain::TickClock;

/// Borrowed, strictly increasing common-clock samples spanning a window from exact rest.
/// Storage belongs to the caller; admission and comparison allocate nothing.
#[derive(Debug, Clone, Copy)]
pub struct TestedTimes<'a> {
    clocks: &'a [TickClock],
}
impl<'a> TestedTimes<'a> {
    /// Require at least two samples, exact initial time zero, and one shared tick profile.
    /// Different clock quanta need an explicitly checked conversion before this interface.
    pub fn new(clocks: &'a [TickClock], maximum_samples: usize) -> Result<Self, VerificationError> {
        if clocks.len() > maximum_samples {
            return Err(VerificationError::CapacityExceeded);
        }
        if clocks.len() < 2 {
            return Err(VerificationError::InvalidTimes);
        }
        let initial = clocks[0];
        if initial.elapsed() != 0 {
            return Err(VerificationError::InvalidTimes);
        }
        let mut previous = 0;
        for clock in &clocks[1..] {
            if clock.exponent() != initial.exponent()
                || clock.target() != initial.target()
                || clock.elapsed() <= previous
            {
                return Err(VerificationError::InvalidTimes);
            }
            previous = clock.elapsed();
        }
        Ok(Self { clocks })
    }

    /// Exact times covered by measurements; no interpolation claim is added.
    pub fn as_slice(self) -> &'a [TickClock] {
        self.clocks
    }

    /// Require a strict superset on the identical window and exact tick profile.
    /// An extension of the endpoint is a different window, not a sampling refinement.
    /// The merge scan inspects at most the number of supplied fine samples.
    pub fn refines(self, coarse: Self) -> bool {
        if self.clocks.len() <= coarse.clocks.len() || self.clocks.last() != coarse.clocks.last() {
            return false;
        }
        let mut matched = 0;
        for clock in self.clocks {
            if *clock == coarse.clocks[matched] {
                matched += 1;
                if matched == coarse.clocks.len() {
                    return true;
                }
            }
        }
        false
    }
}
