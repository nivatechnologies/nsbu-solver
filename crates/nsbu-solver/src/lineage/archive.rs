//! Bounded declaration-event archives; decoded bytes do not authenticate a physical state.
use super::{Digest, LineageError, Origin, Profile, Record, RecordId, Registry};
use crate::domain::TickClock;
mod codec;
use codec::{event_len, Cursor, Output};

const MAGIC: &[u8; 8] = b"NSBULN01";
const VERSION: u16 = 1;

/// A parent identity retained even when an attempted append was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef {
    /// Registry identity named by the declaration.
    pub registry: Digest,
    /// Exact append position in that registry.
    pub index: u128,
}

/// Declared ancestry, including transfer-report identity but no report contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchivedOrigin {
    /// A declaration at exact rest.
    FromRest,
    /// A non-frontier reference declaration.
    LocalReference,
    /// A continuation naming its parent record.
    Continued(ParentRef),
    /// A profile transfer naming its parent and retained report identity.
    Transferred {
        /// The parent declaration.
        parent: ParentRef,
        /// An external error-report identity; its contents are outside this archive.
        errors: Digest,
    },
}

/// The recorded result of one append attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    /// The declaration was recorded at this exact registry position.
    Recorded(ParentRef),
    /// The declaration was refused with this registry error.
    Refused(LineageError),
}

/// The recorded result of one force-invalidation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationOutcome {
    /// This many live records changed to invalid.
    Changed(u128),
    /// The invalidation was refused with this registry error.
    Refused(LineageError),
}

/// One ordered registry operation. Refused operations are retained because they consume budget.
#[allow(clippy::large_enum_variant)] // Inline events keep decoding to one bounded allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// An append attempt and its observed result.
    Append {
        /// Immutable problem, execution, and force identities.
        profile: Profile,
        /// Exact declared endpoint.
        clock: TickClock,
        /// Declared ancestry.
        origin: ArchivedOrigin,
        /// Expected checked-registry result.
        outcome: AppendOutcome,
    },
    /// A force invalidation attempt and its observed result.
    Invalidate {
        /// The force identity whose evidence was revised.
        force: Digest,
        /// Original finite forward-scan allowance.
        maximum_visits: u128,
        /// Expected checked-registry result.
        outcome: InvalidationOutcome,
    },
}

/// Caller-owned declaration log. It contains no state image or provenance authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageLog {
    identity: Digest,
    initial_attempts: u128,
    capacity: u128,
    events: Vec<Event>,
}
impl LineageLog {
    /// Preserve the externally unique identity, finite append allowance, and exact event order.
    pub fn new(
        identity: Digest,
        initial_attempts: u128,
        capacity: u128,
        events: Vec<Event>,
    ) -> Self {
        Self {
            identity,
            initial_attempts,
            capacity,
            events,
        }
    }
    /// Externally unique identity required by replay.
    pub fn identity(&self) -> Digest {
        self.identity
    }
    /// Original finite append-attempt allowance, including refused appends.
    pub fn initial_attempts(&self) -> u128 {
        self.initial_attempts
    }
    /// Exact caller-reserved registry capacity used by the original trace.
    pub fn capacity(&self) -> u128 {
        self.capacity
    }
    /// Ordered immutable declarations and invalidation attempts.
    pub fn events(&self) -> &[Event] {
        &self.events
    }
}

/// Canonical, versioned, independently owned lineage declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageArchive {
    log: LineageLog,
}

/// Refusal to encode, decode, or checked-replay an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveError {
    /// A byte, event, output, scratch, or owned-storage cap was insufficient.
    ResourceLimit,
    /// Bytes or declared outcomes were not a canonical checked-registry trace.
    InvalidEncoding,
    /// The supplied replay storage was unacceptable to the checked registry.
    Replay(LineageError),
}

impl LineageArchive {
    /// Retain a log for canonical writing. This does not assert its events can be replayed.
    pub fn from_log(log: LineageLog) -> Self {
        Self { log }
    }
    /// Borrow the decoded or supplied declaration log.
    pub fn log(&self) -> &LineageLog {
        &self.log
    }

    /// Decode only after byte, event, and owned-storage caps have been checked.
    pub fn read(
        bytes: &[u8],
        maximum_bytes: usize,
        maximum_events: usize,
        maximum_storage: usize,
    ) -> Result<Self, ArchiveError> {
        if bytes.len() > maximum_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut cursor = Cursor { bytes, position: 0 };
        if cursor.take(8)? != MAGIC || cursor.u16()? != VERSION {
            return Err(ArchiveError::InvalidEncoding);
        }
        let identity = cursor.digest()?;
        let attempts = cursor.u128()?;
        let capacity = cursor.u128()?;
        let count = checked_count(cursor.u128()?, maximum_events)?;
        if count > cursor.bytes.len().saturating_sub(cursor.position) / 51 {
            return Err(ArchiveError::InvalidEncoding);
        }
        storage_for(count, maximum_storage)?;
        let mut events = Vec::new();
        events
            .try_reserve_exact(count)
            .map_err(|_| ArchiveError::ResourceLimit)?;
        for _ in 0..count {
            events.push(cursor.event()?);
        }
        if cursor.position != bytes.len() {
            return Err(ArchiveError::InvalidEncoding);
        }
        Ok(Self::from_log(LineageLog::new(
            identity, attempts, capacity, events,
        )))
    }

    /// Exact canonical size. The calculation completes before an output buffer is touched.
    pub fn encoded_len(log: &LineageLog) -> Result<usize, ArchiveError> {
        log.events.iter().try_fold(90usize, |size, event| {
            size.checked_add(event_len(*event))
                .ok_or(ArchiveError::ResourceLimit)
        })
    }

    /// Write canonical bytes. A short output is unchanged.
    pub fn write(log: &LineageLog, output: &mut [u8]) -> Result<usize, ArchiveError> {
        let required = Self::encoded_len(log)?;
        if output.len() < required {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut out = Output {
            bytes: output,
            position: 0,
        };
        out.put(MAGIC);
        out.u16(VERSION);
        out.digest(log.identity);
        out.u128(log.initial_attempts);
        out.u128(log.capacity);
        out.u128(log.events.len() as u128);
        for event in &log.events {
            out.event(*event);
        }
        Ok(out.position)
    }

    /// Replay every event in order through `Registry`, checking each declared result.
    /// `ids` is caller scratch for recorded ids; it is unspecified after any replay error.
    pub fn replay<'a>(
        &self,
        entries: &'a mut [Option<Record>],
        ids: &mut [Option<RecordId>],
    ) -> Result<Registry<'a>, ArchiveError> {
        let attempts =
            usize::try_from(self.log.initial_attempts).map_err(|_| ArchiveError::ResourceLimit)?;
        let capacity =
            usize::try_from(self.log.capacity).map_err(|_| ArchiveError::ResourceLimit)?;
        if entries.len() != capacity {
            return Err(ArchiveError::ResourceLimit);
        }
        if ids.len() < recorded_count(&self.log.events) {
            return Err(ArchiveError::ResourceLimit);
        }
        if entries.len() < ids.len().min(recorded_count(&self.log.events)) {
            return Err(ArchiveError::ResourceLimit);
        }
        ids.fill(None);
        let mut registry =
            Registry::new(self.log.identity, entries, attempts).map_err(ArchiveError::Replay)?;
        let mut next = 0usize;
        for event in &self.log.events {
            match *event {
                Event::Append {
                    profile,
                    clock,
                    origin,
                    outcome,
                } => {
                    if let Some(id) = replay_append(&mut registry, profile, clock, origin, outcome)?
                    {
                        ids[next] = Some(id);
                        next += 1;
                    }
                }
                Event::Invalidate {
                    force,
                    maximum_visits,
                    outcome,
                } => {
                    replay_invalidation(&mut registry, force, maximum_visits, outcome)?;
                }
            }
        }
        Ok(registry)
    }
}

fn replay_append(
    registry: &mut Registry<'_>,
    profile: Profile,
    clock: TickClock,
    origin: ArchivedOrigin,
    outcome: AppendOutcome,
) -> Result<Option<RecordId>, ArchiveError> {
    match (
        registry.append(profile, clock, replay_origin(origin)?),
        outcome,
    ) {
        (Ok(id), AppendOutcome::Recorded(expected)) if matches_id(id, expected) => Ok(Some(id)),
        (Err(error), AppendOutcome::Refused(expected)) if error == expected => Ok(None),
        _ => Err(ArchiveError::InvalidEncoding),
    }
}

fn replay_invalidation(
    registry: &mut Registry<'_>,
    force: Digest,
    maximum_visits: u128,
    outcome: InvalidationOutcome,
) -> Result<(), ArchiveError> {
    let visits = usize::try_from(maximum_visits).map_err(|_| ArchiveError::ResourceLimit)?;
    match (registry.invalidate_force(force, visits), outcome) {
        (Ok(changed), InvalidationOutcome::Changed(expected)) if changed as u128 == expected => {
            Ok(())
        }
        (Err(error), InvalidationOutcome::Refused(expected)) if error == expected => Ok(()),
        _ => Err(ArchiveError::InvalidEncoding),
    }
}

fn replay_origin(origin: ArchivedOrigin) -> Result<Origin, ArchiveError> {
    let parent = |reference: ParentRef| -> Result<RecordId, ArchiveError> {
        let index = usize::try_from(reference.index).map_err(|_| ArchiveError::InvalidEncoding)?;
        Ok(RecordId {
            registry: reference.registry,
            index,
        })
    };
    match origin {
        ArchivedOrigin::FromRest => Ok(Origin::FromRest),
        ArchivedOrigin::LocalReference => Ok(Origin::LocalReference),
        ArchivedOrigin::Continued(reference) => Ok(Origin::Continued(parent(reference)?)),
        ArchivedOrigin::Transferred {
            parent: reference,
            errors,
        } => Ok(Origin::Transferred {
            parent: parent(reference)?,
            errors,
        }),
    }
}

fn matches_id(id: RecordId, expected: ParentRef) -> bool {
    id.registry == expected.registry && id.index as u128 == expected.index
}
fn recorded_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::Append {
                    outcome: AppendOutcome::Recorded(_),
                    ..
                }
            )
        })
        .count()
}
fn checked_count(count: u128, maximum: usize) -> Result<usize, ArchiveError> {
    let count = usize::try_from(count).map_err(|_| ArchiveError::ResourceLimit)?;
    if count > maximum {
        Err(ArchiveError::ResourceLimit)
    } else {
        Ok(count)
    }
}
fn storage_for(count: usize, maximum: usize) -> Result<(), ArchiveError> {
    count
        .checked_mul(std::mem::size_of::<Event>())
        .and_then(|v| v.checked_add(std::mem::size_of::<LineageArchive>()))
        .filter(|&v| v <= maximum)
        .ok_or(ArchiveError::ResourceLimit)
        .map(|_| ())
}
