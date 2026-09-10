//! Private byte framing for the lineage event container.
use super::{AppendOutcome, ArchiveError, ArchivedOrigin, Event, InvalidationOutcome, ParentRef};
use crate::{
    domain::TickClock,
    lineage::{Digest, LineageError, Profile},
};

pub(super) fn event_len(event: Event) -> usize {
    match event {
        Event::Append {
            origin, outcome, ..
        } => 149 + origin_len(origin) + outcome_len(outcome),
        Event::Invalidate { outcome, .. } => 49 + invalidation_len(outcome),
    }
}
fn origin_len(origin: ArchivedOrigin) -> usize {
    match origin {
        ArchivedOrigin::FromRest | ArchivedOrigin::LocalReference => 1,
        ArchivedOrigin::Continued(_) => 49,
        ArchivedOrigin::Transferred { .. } => 81,
    }
}
fn outcome_len(outcome: AppendOutcome) -> usize {
    match outcome {
        AppendOutcome::Recorded(_) => 49,
        AppendOutcome::Refused(_) => 2,
    }
}
fn invalidation_len(outcome: InvalidationOutcome) -> usize {
    match outcome {
        InvalidationOutcome::Changed(_) => 17,
        InvalidationOutcome::Refused(_) => 2,
    }
}

pub(super) struct Output<'a> {
    pub(super) bytes: &'a mut [u8],
    pub(super) position: usize,
}
impl Output<'_> {
    pub(super) fn put(&mut self, value: &[u8]) {
        let end = self.position + value.len();
        self.bytes[self.position..end].copy_from_slice(value);
        self.position = end;
    }
    pub(super) fn u16(&mut self, value: u16) {
        self.put(&value.to_le_bytes());
    }
    fn u8(&mut self, value: u8) {
        self.put(&[value]);
    }
    pub(super) fn u128(&mut self, value: u128) {
        self.put(&value.to_le_bytes());
    }
    pub(super) fn digest(&mut self, value: Digest) {
        self.put(&value.bytes());
    }
    fn parent(&mut self, value: ParentRef) {
        self.digest(value.registry);
        self.u128(value.index);
    }
    fn error(&mut self, value: LineageError) {
        self.u8(error_tag(value));
    }
    fn origin(&mut self, value: ArchivedOrigin) {
        match value {
            ArchivedOrigin::FromRest => self.u8(0),
            ArchivedOrigin::LocalReference => self.u8(1),
            ArchivedOrigin::Continued(parent) => {
                self.u8(2);
                self.parent(parent);
            }
            ArchivedOrigin::Transferred { parent, errors } => {
                self.u8(3);
                self.parent(parent);
                self.digest(errors);
            }
        }
    }
    pub(super) fn event(&mut self, value: Event) {
        match value {
            Event::Append {
                profile,
                clock,
                origin,
                outcome,
            } => self.append(profile, clock, origin, outcome),
            Event::Invalidate {
                force,
                maximum_visits,
                outcome,
            } => self.invalidate(force, maximum_visits, outcome),
        }
    }
    fn append(
        &mut self,
        profile: Profile,
        clock: TickClock,
        origin: ArchivedOrigin,
        outcome: AppendOutcome,
    ) {
        self.u8(0);
        self.digest(profile.problem);
        self.digest(profile.execution);
        self.digest(profile.force);
        self.put(&clock.exponent().to_le_bytes());
        self.u128(clock.target());
        self.u128(clock.elapsed());
        self.u128(clock.remaining());
        self.origin(origin);
        match outcome {
            AppendOutcome::Recorded(id) => {
                self.u8(0);
                self.parent(id);
            }
            AppendOutcome::Refused(error) => {
                self.u8(1);
                self.error(error);
            }
        }
    }
    fn invalidate(&mut self, force: Digest, maximum_visits: u128, outcome: InvalidationOutcome) {
        self.u8(1);
        self.digest(force);
        self.u128(maximum_visits);
        match outcome {
            InvalidationOutcome::Changed(changed) => {
                self.u8(0);
                self.u128(changed);
            }
            InvalidationOutcome::Refused(error) => {
                self.u8(1);
                self.error(error);
            }
        }
    }
}

pub(super) struct Cursor<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) position: usize,
}
impl<'a> Cursor<'a> {
    pub(super) fn take(&mut self, count: usize) -> Result<&'a [u8], ArchiveError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(ArchiveError::InvalidEncoding)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(ArchiveError::InvalidEncoding)?;
        self.position = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ArchiveError> {
        let mut value = [0; N];
        value.copy_from_slice(self.take(N)?);
        Ok(value)
    }
    pub(super) fn u16(&mut self) -> Result<u16, ArchiveError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u8(&mut self) -> Result<u8, ArchiveError> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn u128(&mut self) -> Result<u128, ArchiveError> {
        Ok(u128::from_le_bytes(self.array()?))
    }
    pub(super) fn digest(&mut self) -> Result<Digest, ArchiveError> {
        Digest::new(self.array()?).map_err(|_| ArchiveError::InvalidEncoding)
    }
    fn parent(&mut self) -> Result<ParentRef, ArchiveError> {
        Ok(ParentRef {
            registry: self.digest()?,
            index: self.u128()?,
        })
    }
    fn error(&mut self) -> Result<LineageError, ArchiveError> {
        error_from_tag(self.u8()?)
    }
    fn origin(&mut self) -> Result<ArchivedOrigin, ArchiveError> {
        match self.u8()? {
            0 => Ok(ArchivedOrigin::FromRest),
            1 => Ok(ArchivedOrigin::LocalReference),
            2 => Ok(ArchivedOrigin::Continued(self.parent()?)),
            3 => Ok(ArchivedOrigin::Transferred {
                parent: self.parent()?,
                errors: self.digest()?,
            }),
            _ => Err(ArchiveError::InvalidEncoding),
        }
    }
    pub(super) fn event(&mut self) -> Result<Event, ArchiveError> {
        match self.u8()? {
            0 => self.append(),
            1 => self.invalidate(),
            _ => Err(ArchiveError::InvalidEncoding),
        }
    }
    fn append(&mut self) -> Result<Event, ArchiveError> {
        let profile = Profile {
            problem: self.digest()?,
            execution: self.digest()?,
            force: self.digest()?,
        };
        let clock = TickClock::restore(
            i32::from_le_bytes(self.array()?),
            self.u128()?,
            self.u128()?,
            self.u128()?,
        )
        .map_err(|_| ArchiveError::InvalidEncoding)?;
        let origin = self.origin()?;
        let outcome = match self.u8()? {
            0 => AppendOutcome::Recorded(self.parent()?),
            1 => AppendOutcome::Refused(self.error()?),
            _ => return Err(ArchiveError::InvalidEncoding),
        };
        Ok(Event::Append {
            profile,
            clock,
            origin,
            outcome,
        })
    }
    fn invalidate(&mut self) -> Result<Event, ArchiveError> {
        let force = self.digest()?;
        let maximum_visits = self.u128()?;
        let outcome = match self.u8()? {
            0 => InvalidationOutcome::Changed(self.u128()?),
            1 => InvalidationOutcome::Refused(self.error()?),
            _ => return Err(ArchiveError::InvalidEncoding),
        };
        Ok(Event::Invalidate {
            force,
            maximum_visits,
            outcome,
        })
    }
}

fn error_tag(value: LineageError) -> u8 {
    match value {
        LineageError::MissingIdentity => 0,
        LineageError::CapacityExceeded => 1,
        LineageError::OccupiedStorage => 2,
        LineageError::UnknownParent => 3,
        LineageError::InvalidatedParent => 4,
        LineageError::DifferentProblem => 5,
        LineageError::DifferentProfile => 6,
        LineageError::InvalidClock => 7,
    }
}
fn error_from_tag(value: u8) -> Result<LineageError, ArchiveError> {
    match value {
        0 => Ok(LineageError::MissingIdentity),
        1 => Ok(LineageError::CapacityExceeded),
        2 => Ok(LineageError::OccupiedStorage),
        3 => Ok(LineageError::UnknownParent),
        4 => Ok(LineageError::InvalidatedParent),
        5 => Ok(LineageError::DifferentProblem),
        6 => Ok(LineageError::DifferentProfile),
        7 => Ok(LineageError::InvalidClock),
        _ => Err(ArchiveError::InvalidEncoding),
    }
}
