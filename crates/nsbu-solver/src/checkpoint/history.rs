//! Versioned raw-log encoding and bounded replay; no physical provenance is imported.
use super::{
    bytes::{put, Cursor},
    record, CheckpointError,
};
use crate::{
    domain::TickClock,
    experiment::{
        control::{Configuration, Controller},
        log::{AttemptRecord, RunHistory},
    },
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    storage::reserved,
};
const MAGIC: &[u8; 8] = b"NSBUHR01";
const HEADER: usize = 157;

/// Exact byte count for a complete raw log, without allocating an encoded copy.
pub fn encoded_len(history: &RunHistory) -> Result<usize, CheckpointError> {
    history
        .records()
        .len()
        .checked_mul(record::WIDTH)
        .and_then(|size| size.checked_add(HEADER))
        .ok_or(CheckpointError::ResourceLimit)
}
/// Preflight the complete output; preserve unused suffix bytes and refuse a short buffer unchanged.
pub fn write(history: &RunHistory, output: &mut [u8]) -> Result<usize, CheckpointError> {
    let required = encoded_len(history)?;
    if output.len() < required {
        return Err(CheckpointError::ResourceLimit);
    }
    let control = history.controller();
    let config = control.configuration();
    let clock = control.clock();
    let initial = TickClock::from_rest(clock.exponent(), clock.target())
        .map_err(CheckpointError::InvalidHistory)?;
    let mut position = 0;
    put(output, &mut position, MAGIC);
    record::clock(output, &mut position, initial);
    let method = match config.method {
        Method::CoxMatthews => 1,
        Method::HochbruckOstermann => 2,
    };
    put(output, &mut position, &[method]);
    for value in [
        config.limits.endpoint,
        config.limits.step_ticks,
        config.limits.maximum_attempts as u128,
    ] {
        put(output, &mut position, &value.to_le_bytes());
    }
    for value in config
        .tolerances
        .absolute
        .into_iter()
        .chain(config.tolerances.relative)
    {
        put(output, &mut position, &value.to_bits().to_le_bytes());
    }
    put(
        output,
        &mut position,
        &(history.records().len() as u128).to_le_bytes(),
    );
    for entry in history.records() {
        record::write(*entry, output, &mut position);
    }
    Ok(position)
}

/// Peak owned history/replay-buffer storage. Caller allocator overhead is additional.
pub fn reservation(config: Configuration, records: usize) -> Result<usize, CheckpointError> {
    let history = RunHistory::reservation(config).map_err(CheckpointError::InvalidHistory)?;
    records
        .checked_mul(std::mem::size_of::<AttemptRecord>())
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Vec<AttemptRecord>>()))
        .and_then(|bytes| bytes.checked_add(history))
        .filter(|bytes| *bytes <= isize::MAX as usize)
        .ok_or(CheckpointError::ResourceLimit)
}
/// Bound bytes, future attempt allowance, replay visits and peak storage before allocation.
/// Reconstruct compensated arithmetic in original order; this does not authenticate a trajectory.
pub fn read(
    bytes: &[u8],
    maximum_bytes: usize,
    maximum_records: usize,
    storage_cap: usize,
) -> Result<RunHistory, CheckpointError> {
    if bytes.len() > maximum_bytes {
        return Err(CheckpointError::ResourceLimit);
    }
    let mut cursor = Cursor { remaining: bytes };
    if cursor.take(8)? != MAGIC {
        return Err(CheckpointError::InvalidEncoding);
    }
    let clock = record::read_clock(&mut cursor)?;
    let config = configuration(&mut cursor)?;
    let count = size(&mut cursor)?;
    if count > config.limits.maximum_attempts || config.limits.maximum_attempts > maximum_records {
        return Err(CheckpointError::ResourceLimit);
    }
    let expected = count
        .checked_mul(record::WIDTH)
        .ok_or(CheckpointError::ResourceLimit)?;
    if cursor.remaining.len() != expected {
        return Err(CheckpointError::InvalidEncoding);
    }
    Controller::new(clock, config).map_err(CheckpointError::InvalidHistory)?;
    if reservation(config, count)? > storage_cap {
        return Err(CheckpointError::ResourceLimit);
    }
    let mut records = reserved(count).map_err(CheckpointError::InvalidHistory)?;
    for _ in 0..count {
        records.push(record::read(&mut cursor)?);
    }
    RunHistory::replay(clock, config, &records, storage_cap)
        .map_err(CheckpointError::InvalidHistory)
}
fn configuration(cursor: &mut Cursor<'_>) -> Result<Configuration, CheckpointError> {
    let method = match cursor.take(1)?[0] {
        1 => Method::CoxMatthews,
        2 => Method::HochbruckOstermann,
        _ => return Err(CheckpointError::InvalidEncoding),
    };
    let endpoint = u128::from_le_bytes(cursor.array()?);
    let step_ticks = u128::from_le_bytes(cursor.array()?);
    let maximum_attempts = size(cursor)?;
    let values = record::floats::<4>(cursor)?;
    Ok(Configuration {
        method,
        limits: RunLimits {
            endpoint,
            step_ticks,
            maximum_attempts,
        },
        tolerances: Tolerances {
            absolute: [values[0], values[1]],
            relative: [values[2], values[3]],
        },
    })
}
fn size(cursor: &mut Cursor<'_>) -> Result<usize, CheckpointError> {
    usize::try_from(u128::from_le_bytes(cursor.array()?))
        .map_err(|_| CheckpointError::ResourceLimit)
}
