//! Lineage archives replay declarations and refusals without asserting physical provenance.
use nsbu_solver::{
    domain::TickClock,
    lineage::{
        archive::{
            AppendOutcome, ArchiveError, ArchivedOrigin, Event, InvalidationOutcome,
            LineageArchive, LineageLog, ParentRef,
        },
        Digest, LineageError, Profile,
    },
};

fn digest(byte: u8) -> Digest {
    Digest::new([byte; 32]).unwrap()
}
fn profile(force: u8) -> Profile {
    Profile {
        problem: digest(1),
        execution: digest(2),
        force: digest(force),
    }
}
fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-30, u128::MAX, elapsed, u128::MAX - elapsed).unwrap()
}
fn id(index: u128) -> ParentRef {
    ParentRef {
        registry: digest(7),
        index,
    }
}

fn trace() -> LineageLog {
    LineageLog::new(
        digest(7),
        4,
        3,
        vec![
            Event::Append {
                profile: profile(3),
                clock: clock(0),
                origin: ArchivedOrigin::FromRest,
                outcome: AppendOutcome::Recorded(id(0)),
            },
            Event::Append {
                profile: profile(8),
                clock: clock(4),
                origin: ArchivedOrigin::Continued(id(0)),
                outcome: AppendOutcome::Recorded(id(1)),
            },
            Event::Invalidate {
                force: digest(3),
                maximum_visits: 1,
                outcome: InvalidationOutcome::Refused(LineageError::CapacityExceeded),
            },
            Event::Invalidate {
                force: digest(3),
                maximum_visits: 2,
                outcome: InvalidationOutcome::Changed(2),
            },
            Event::Append {
                profile: profile(8),
                clock: clock(8),
                origin: ArchivedOrigin::Continued(id(1)),
                outcome: AppendOutcome::Refused(LineageError::InvalidatedParent),
            },
        ],
    )
}

fn bytes(log: &LineageLog) -> Vec<u8> {
    let mut out = vec![0x91; LineageArchive::encoded_len(log).unwrap() + 1];
    let size = LineageArchive::encoded_len(log).unwrap();
    assert_eq!(
        LineageArchive::write(log, &mut out[..size - 1]),
        Err(ArchiveError::ResourceLimit)
    );
    assert!(out.iter().all(|byte| *byte == 0x91));
    assert_eq!(LineageArchive::write(log, &mut out), Ok(size));
    assert_eq!(out[size], 0x91);
    out.truncate(size);
    out
}

#[test]
fn canonical_round_trip_replays_interleaved_refusals_and_invalidation() {
    let log = trace();
    let encoded = bytes(&log);
    let archive = LineageArchive::read(&encoded, encoded.len(), 5, usize::MAX).unwrap();
    assert_eq!(archive.log(), &log);
    assert_eq!(bytes(archive.log()), encoded);
    let mut records = [None; 3];
    let mut ids = [None; 2];
    let registry = archive.replay(&mut records, &mut ids).unwrap();
    assert!(!registry.get(ids[0].unwrap()).unwrap().is_valid());
    assert!(!registry.get(ids[1].unwrap()).unwrap().is_valid());
    assert_eq!(registry.attempts_left(), 1);
}

#[test]
fn caps_corruption_missing_ancestor_and_bad_order_refuse_replay() {
    let encoded = bytes(&trace());
    assert_eq!(
        LineageArchive::read(&encoded, encoded.len() - 1, 5, usize::MAX),
        Err(ArchiveError::ResourceLimit)
    );
    assert_eq!(
        LineageArchive::read(&encoded, encoded.len(), 4, usize::MAX),
        Err(ArchiveError::ResourceLimit)
    );
    assert_eq!(
        LineageArchive::read(&encoded, encoded.len(), 5, 0),
        Err(ArchiveError::ResourceLimit)
    );
    let mut corrupt = encoded;
    corrupt[0] ^= 1;
    assert_eq!(
        LineageArchive::read(&corrupt, corrupt.len(), 5, usize::MAX),
        Err(ArchiveError::InvalidEncoding)
    );
    let mut implausible = bytes(&trace());
    implausible[74..90].copy_from_slice(&100_u128.to_le_bytes());
    assert_eq!(
        LineageArchive::read(&implausible, implausible.len(), 100, usize::MAX),
        Err(ArchiveError::InvalidEncoding)
    );
    let missing = LineageArchive::from_log(LineageLog::new(
        digest(7),
        1,
        1,
        vec![Event::Append {
            profile: profile(3),
            clock: clock(4),
            origin: ArchivedOrigin::Continued(id(4)),
            outcome: AppendOutcome::Refused(LineageError::UnknownParent),
        }],
    ));
    let mut records = [None; 1];
    let mut ids = [None; 1];
    assert!(missing.replay(&mut records, &mut ids).is_ok());
    let bad_order = LineageArchive::from_log(LineageLog::new(
        digest(7),
        1,
        1,
        vec![Event::Append {
            profile: profile(3),
            clock: clock(4),
            origin: ArchivedOrigin::Continued(id(1)),
            outcome: AppendOutcome::Recorded(id(0)),
        }],
    ));
    assert!(matches!(
        bad_order.replay(&mut records, &mut ids),
        Err(ArchiveError::InvalidEncoding)
    ));
}

#[test]
fn replay_requires_the_declared_capacity_empty_storage_and_id_scratch() {
    let archive = LineageArchive::from_log(trace());
    let mut short_records = [None; 2];
    let mut ids = [None; 2];
    assert!(matches!(
        archive.replay(&mut short_records, &mut ids),
        Err(ArchiveError::ResourceLimit)
    ));
    let mut records = [None; 3];
    let mut short_ids = [None; 1];
    assert!(matches!(
        archive.replay(&mut records, &mut short_ids),
        Err(ArchiveError::ResourceLimit)
    ));
    let mut complete_ids = [None; 2];
    {
        let _registry = archive.replay(&mut records, &mut complete_ids).unwrap();
    }
    assert!(matches!(
        archive.replay(&mut records, &mut complete_ids),
        Err(ArchiveError::Replay(LineageError::OccupiedStorage))
    ));
}

#[test]
fn replay_rejects_mismatched_declared_results_and_exhausted_allowance() {
    let mismatch = LineageArchive::from_log(LineageLog::new(
        digest(7),
        1,
        1,
        vec![Event::Append {
            profile: profile(3),
            clock: clock(0),
            origin: ArchivedOrigin::FromRest,
            outcome: AppendOutcome::Refused(LineageError::InvalidClock),
        }],
    ));
    let mut records = [None; 1];
    let mut ids = [None; 1];
    assert!(matches!(
        mismatch.replay(&mut records, &mut ids),
        Err(ArchiveError::InvalidEncoding)
    ));
    records.fill(None);
    let exhausted = LineageArchive::from_log(LineageLog::new(
        digest(7),
        0,
        1,
        vec![Event::Append {
            profile: profile(3),
            clock: clock(0),
            origin: ArchivedOrigin::FromRest,
            outcome: AppendOutcome::Recorded(id(0)),
        }],
    ));
    assert!(matches!(
        exhausted.replay(&mut records, &mut ids),
        Err(ArchiveError::InvalidEncoding)
    ));
    records.fill(None);
    let invalidation_mismatch = LineageArchive::from_log(LineageLog::new(
        digest(7),
        1,
        1,
        vec![
            Event::Append {
                profile: profile(3),
                clock: clock(0),
                origin: ArchivedOrigin::FromRest,
                outcome: AppendOutcome::Recorded(id(0)),
            },
            Event::Invalidate {
                force: digest(9),
                maximum_visits: 1,
                outcome: InvalidationOutcome::Changed(1),
            },
        ],
    ));
    assert!(matches!(
        invalidation_mismatch.replay(&mut records, &mut ids),
        Err(ArchiveError::InvalidEncoding)
    ));
}
