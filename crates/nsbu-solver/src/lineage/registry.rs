use super::{Digest, LineageError, Origin, Profile, Record, RecordId};
use crate::domain::TickClock;

/// Caller-reserved, append-only ancestry. The registry never allocates or edits a solver state.
/// External import must separately authenticate manifests and verify physical-state provenance.
pub struct Registry<'a> {
    identity: Digest,
    entries: &'a mut [Option<Record>],
    len: usize,
    attempts: usize,
}
impl<'a> Registry<'a> {
    /// Preflight caller storage, excluding its allocator overhead and external report payloads.
    pub fn reservation(capacity: usize) -> Result<usize, LineageError> {
        capacity
            .checked_mul(std::mem::size_of::<Option<Record>>())
            .filter(|&bytes| bytes <= isize::MAX as usize)
            .ok_or(LineageError::CapacityExceeded)
    }
    /// Borrow empty storage and a finite append-attempt allowance. Identity must be unique.
    pub fn new(
        identity: Digest,
        entries: &'a mut [Option<Record>],
        attempts: usize,
    ) -> Result<Self, LineageError> {
        if entries.iter().any(Option::is_some) {
            return Err(LineageError::OccupiedStorage);
        }
        Ok(Self {
            identity,
            entries,
            len: 0,
            attempts,
        })
    }
    /// Remaining append attempts; invalid declarations consume work without adding records.
    pub fn attempts_left(&self) -> usize {
        self.attempts
    }
    /// Retrieve only an existing position belonging to this registry.
    pub fn get(&self, id: RecordId) -> Result<Record, LineageError> {
        if id.registry != self.identity {
            return Err(LineageError::UnknownParent);
        }
        self.entries
            .get(id.index)
            .copied()
            .flatten()
            .ok_or(LineageError::UnknownParent)
    }
    /// Record ancestry after all checks; failed admission preserves every earlier record.
    pub fn append(
        &mut self,
        profile: Profile,
        clock: TickClock,
        origin: Origin,
    ) -> Result<RecordId, LineageError> {
        if self.attempts == 0 {
            return Err(LineageError::CapacityExceeded);
        }
        self.attempts -= 1;
        if self.len == self.entries.len() {
            return Err(LineageError::CapacityExceeded);
        }
        let direct = match origin.parent() {
            Some(id) => self.check_parent(profile, clock, origin, self.get(id)?)?,
            None => {
                if origin == Origin::FromRest && clock.elapsed() != 0 {
                    return Err(LineageError::InvalidClock);
                }
                origin == Origin::FromRest
            }
        };
        let id = RecordId {
            registry: self.identity,
            index: self.len,
        };
        self.entries[self.len] = Some(Record {
            profile,
            clock,
            origin,
            direct,
            valid: true,
        });
        self.len += 1;
        Ok(id)
    }
    fn check_parent(
        &self,
        profile: Profile,
        clock: TickClock,
        origin: Origin,
        parent: Record,
    ) -> Result<bool, LineageError> {
        if !parent.valid {
            return Err(LineageError::InvalidatedParent);
        }
        if !profile.same_problem(parent.profile) {
            return Err(LineageError::DifferentProblem);
        }
        if clock.target() != parent.clock.target() || clock.exponent() != parent.clock.exponent() {
            return Err(LineageError::InvalidClock);
        }
        match origin {
            Origin::Continued(_) => {
                if profile.execution != parent.profile.execution {
                    return Err(LineageError::DifferentProfile);
                }
                if clock.elapsed() <= parent.clock.elapsed() {
                    return Err(LineageError::InvalidClock);
                }
                Ok(parent.direct)
            }
            _ => {
                if clock != parent.clock {
                    return Err(LineageError::InvalidClock);
                }
                Ok(false)
            }
        }
    }
    /// Invalidate every use of a force artifact and all descendants in one bounded forward pass.
    /// This applies when revised force evidence exceeds an earlier budget; validity never revives.
    /// Insufficient visit allowance refuses the entire operation before changing any record.
    pub fn invalidate_force(
        &mut self,
        force: Digest,
        maximum_visits: usize,
    ) -> Result<usize, LineageError> {
        if maximum_visits < self.len {
            return Err(LineageError::CapacityExceeded);
        }
        let mut changed = 0;
        for index in 0..self.len {
            let mut record = self.entries[index].expect("append-only occupied prefix");
            let inherited = match record.origin.parent() {
                Some(id) => !self.get(id)?.valid,
                None => false,
            };
            if record.valid && (record.profile.force == force || inherited) {
                record.valid = false;
                self.entries[index] = Some(record);
                changed += 1;
            }
        }
        Ok(changed)
    }
}
