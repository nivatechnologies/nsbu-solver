//! Verify bounded archive framing and identity before allocating decoded state.
use super::*;
use crate::v2_run::work;

struct Header {
    physical_size: usize,
    history_size: usize,
    records: usize,
    observer: BalanceObserverWork,
}

pub(super) fn read(
    bytes: &[u8],
    expected: Plan,
    maximum: usize,
    cap: usize,
) -> Result<ImportedV2Run, CheckpointError> {
    let body = verified_body(bytes, maximum)?;
    let mut cursor = Cursor { remaining: body };
    identity(&mut cursor, expected)?;
    let header = header(&mut cursor)?;
    admit_frame(body.len(), &header, expected, cap)?;
    decode(&mut cursor, header, expected)
}

fn verified_body(bytes: &[u8], maximum: usize) -> Result<&[u8], CheckpointError> {
    if bytes.len() > maximum || bytes.len() < HASH {
        return Err(CheckpointError::ResourceLimit);
    }
    let (body, claimed) = bytes.split_at(bytes.len() - HASH);
    if Sha256::digest(body).as_slice() != claimed {
        return Err(CheckpointError::HashMismatch);
    }
    Ok(body)
}

fn identity(cursor: &mut Cursor<'_>, expected: Plan) -> Result<(), CheckpointError> {
    if cursor.take(8)? != MAGIC
        || u16::from_le_bytes(cursor.array()?) != VERSION
        || cursor.take(64)? != crate::CASE_SHA256.as_bytes()
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    force_identity(cursor, expected)?;
    let initial = read_clock(cursor)?;
    let length_bits = u64::from_le_bytes(cursor.array()?);
    let advective_bits = u64::from_le_bytes(cursor.array()?);
    let configuration = read_config(cursor)?;
    let settings = expected.settings();
    if initial != settings.initial_clock
        || length_bits != settings.domain.lengths()[0].to_bits()
        || advective_bits != settings.advective_limit.to_bits()
        || !work::same_configuration(settings.configuration, configuration)
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}

fn force_identity(cursor: &mut Cursor<'_>, expected: Plan) -> Result<(), CheckpointError> {
    let settings = expected.settings().force;
    if cursor.take(1)?[0] != u8::from(settings.workers != 0) {
        return Err(CheckpointError::InvalidEncoding);
    }
    for dimension in settings.samples.dimensions() {
        if size(cursor)? != dimension {
            return Err(CheckpointError::InvalidEncoding);
        }
    }
    if size(cursor)? != settings.workers {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}

fn header(cursor: &mut Cursor<'_>) -> Result<Header, CheckpointError> {
    Ok(Header {
        physical_size: size(cursor)?,
        history_size: size(cursor)?,
        records: size(cursor)?,
        observer: observation(cursor)?,
    })
}

fn admit_frame(
    body_size: usize,
    header: &Header,
    plan: Plan,
    cap: usize,
) -> Result<(), CheckpointError> {
    if read_reservation(plan, header.records)? > cap {
        return Err(CheckpointError::ResourceLimit);
    }
    let framed = HEADER
        .checked_add(header.physical_size)
        .and_then(|n| n.checked_add(header.history_size))
        .and_then(|n| n.checked_add(header.records.checked_mul(WORK)?))
        .ok_or(CheckpointError::ResourceLimit)?;
    if body_size != framed
        || header.physical_size != PhysicalArchive::encoded_len_for_plan(plan.resources())?
        || header.history_size > history::maximum_encoded_len(plan.settings().configuration)?
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    crate::smooth_observer::v2::V2Observer::validate_restored(
        plan.settings().domain,
        plan.settings().force,
        plan.observer_samples(),
        header.observer,
    )
    .map_err(CheckpointError::InvalidHistory)?;
    Ok(())
}

fn decode(
    cursor: &mut Cursor<'_>,
    header: Header,
    plan: Plan,
) -> Result<ImportedV2Run, CheckpointError> {
    let physical = PhysicalArchive::read(
        cursor.take(header.physical_size)?,
        plan.resources(),
        header.physical_size,
        UnverifiedPhysical::reservation(plan.resources())
            .map_err(CheckpointError::InvalidHistory)?,
    )?;
    let history = history::read(
        cursor.take(header.history_size)?,
        header.history_size,
        plan.settings().configuration.limits.maximum_attempts,
        history::reservation(plan.settings().configuration, header.records)?,
    )?;
    let work = decode_work(cursor, plan, header.records)?;
    work::validate(plan, physical.state(), &history, &work, header.observer)
        .map_err(CheckpointError::InvalidHistory)?;
    Ok(ImportedV2Run {
        physical,
        history,
        work,
        observer_work: header.observer,
        plan,
    })
}

fn decode_work(
    cursor: &mut Cursor<'_>,
    plan: Plan,
    records: usize,
) -> Result<Vec<AttemptWork>, CheckpointError> {
    let mut entries = work::storage(plan).map_err(CheckpointError::InvalidHistory)?;
    for _ in 0..records {
        entries.push(AttemptWork {
            integration: [size(cursor)?, size(cursor)?, size(cursor)?],
            observation: observation(cursor)?,
        });
    }
    if !cursor.remaining.is_empty() {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(entries)
}

fn observation(cursor: &mut Cursor<'_>) -> Result<BalanceObserverWork, CheckpointError> {
    Ok(BalanceObserverWork {
        samples: size(cursor)?,
        work_units: size(cursor)?,
        scalar_transforms: size(cursor)?,
    })
}
