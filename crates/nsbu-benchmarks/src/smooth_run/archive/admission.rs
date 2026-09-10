use super::{size, Cursor, Header, ImportedSmoothRun, HASH, HEADER, MAGIC, VERSION, WORK};
use crate::{
    smooth::CyclicSine,
    smooth_observer::{BalanceObserver, BalanceObserverWork},
    smooth_run::{
        ledger_bytes, observation::Observation, same_configuration, work_storage, IntegrationWork,
        OwnedPlan,
    },
};
use nsbu_solver::{
    checkpoint::{
        history,
        physical::{PhysicalArchive, UnverifiedPhysical},
        CheckpointError,
    },
    domain::{ResourcePlan, TickClock},
    experiment::{
        control::{Configuration, Controller, Outcome},
        log::RunHistory,
    },
    integrators::forcing::{ForceLimits, PrescribedForce},
    SolverError,
};
use sha2::{Digest, Sha256};

pub(super) fn read(
    bytes: &[u8],
    expected: ResourcePlan,
    maximum: usize,
    cap: usize,
) -> Result<ImportedSmoothRun, CheckpointError> {
    read_profile::<BalanceObserver>(bytes, expected, maximum, cap, 0)
}

pub(in crate::smooth_run) fn read_profile<O: Observation>(
    bytes: &[u8],
    expected: ResourcePlan,
    maximum: usize,
    cap: usize,
    initial_samples: usize,
) -> Result<ImportedSmoothRun, CheckpointError> {
    let body = verified_body(bytes, maximum)?;
    let (mut cursor, header) = decode_header(body)?;
    admit_header::<O>(body, expected, cap, &header)?;
    let physical = read_physical(&mut cursor, expected, cap, &header)?;
    let history = read_history(&mut cursor, cap, &header)?;
    validate_profile(&physical, &history, &header)?;
    let work = read_work(&mut cursor, expected, &history, &header)?;
    finish_import(cursor, expected, &history, &header, initial_samples)?;
    Ok(ImportedSmoothRun {
        physical,
        history,
        work,
        observer_work: header.observer_work,
        configuration: header.configuration,
        initial_clock: header.initial_clock,
        observer_samples: header.observer_samples,
        advective_limit: header.advective_limit,
    })
}

fn verified_body(bytes: &[u8], maximum: usize) -> Result<&[u8], CheckpointError> {
    if bytes.len() > maximum {
        return Err(CheckpointError::ResourceLimit);
    }
    let split = bytes
        .len()
        .checked_sub(HASH)
        .ok_or(CheckpointError::InvalidEncoding)?;
    let (body, claimed) = bytes
        .split_at_checked(split)
        .ok_or(CheckpointError::InvalidEncoding)?;
    if body.len() < HEADER {
        return Err(CheckpointError::InvalidEncoding);
    }
    if Sha256::digest(body).as_slice() != claimed {
        return Err(CheckpointError::HashMismatch);
    }
    Ok(body)
}

fn decode_header(body: &[u8]) -> Result<(Cursor<'_>, Header), CheckpointError> {
    let mut cursor = Cursor { remaining: body };
    decode_prefix(&mut cursor)?;
    let header = Header {
        initial_clock: super::read_clock(&mut cursor)?,
        configuration: super::read_configuration(&mut cursor)?,
        observer_samples: size(&mut cursor)?,
        advective_limit: f64::from_bits(u64::from_le_bytes(cursor.array()?)),
        physical_size: size(&mut cursor)?,
        history_size: size(&mut cursor)?,
        records: size(&mut cursor)?,
        observer_work: read_observer_work(&mut cursor)?,
    };
    valid_header(&header)?;
    Ok((cursor, header))
}
fn decode_prefix(cursor: &mut Cursor<'_>) -> Result<(), CheckpointError> {
    if cursor.take(8)? != MAGIC || u16::from_le_bytes(cursor.array()?) != VERSION {
        return Err(CheckpointError::InvalidEncoding);
    }
    if !matches!(cursor.take(1)?[0], 1 | 2) {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}
fn read_observer_work(cursor: &mut Cursor<'_>) -> Result<BalanceObserverWork, CheckpointError> {
    Ok(BalanceObserverWork {
        samples: size(cursor)?,
        work_units: size(cursor)?,
        scalar_transforms: size(cursor)?,
    })
}
fn valid_header(header: &Header) -> Result<(), CheckpointError> {
    if !header.advective_limit.is_finite() || header.advective_limit <= 0.0 {
        return Err(CheckpointError::InvalidEncoding);
    }
    if !super::is_rest(header.initial_clock)
        || Controller::new(header.initial_clock, header.configuration).is_err()
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}
fn admit_header<O: Observation>(
    body: &[u8],
    expected: ResourcePlan,
    cap: usize,
    header: &Header,
) -> Result<(), CheckpointError> {
    if body.len() != framed_len(header)? {
        return Err(CheckpointError::InvalidEncoding);
    }
    if header.records > header.configuration.limits.maximum_attempts {
        return Err(CheckpointError::ResourceLimit);
    }
    let profile = OwnedPlan::<O>::from_rest(
        expected.domain(),
        header.initial_clock,
        header.configuration,
        header.observer_samples,
        header.advective_limit,
        cap,
    )
    .map_err(CheckpointError::InvalidHistory)?;
    if profile.resources() != expected {
        return Err(CheckpointError::InvalidEncoding);
    }
    if import_peak(expected, header)? > cap {
        return Err(CheckpointError::ResourceLimit);
    }
    Ok(())
}
fn framed_len(header: &Header) -> Result<usize, CheckpointError> {
    let records = header
        .records
        .checked_mul(WORK)
        .ok_or(CheckpointError::ResourceLimit)?;
    let payload = header
        .physical_size
        .checked_add(header.history_size)
        .and_then(|size| size.checked_add(records))
        .ok_or(CheckpointError::ResourceLimit)?;
    HEADER
        .checked_add(payload)
        .ok_or(CheckpointError::ResourceLimit)
}
fn import_peak(expected: ResourcePlan, header: &Header) -> Result<usize, CheckpointError> {
    let physical =
        UnverifiedPhysical::reservation(expected).map_err(|_| CheckpointError::ResourceLimit)?;
    let history = history::reservation(header.configuration, header.records)?;
    let ledger = ledger_bytes(header.configuration.limits.maximum_attempts)
        .map_err(CheckpointError::InvalidHistory)?;
    expected
        .total()
        .checked_add(physical)
        .and_then(|size| size.checked_add(history))
        .and_then(|size| size.checked_add(ledger))
        .and_then(|size| size.checked_add(std::mem::size_of::<ImportedSmoothRun>()))
        .ok_or(CheckpointError::ResourceLimit)
}
fn read_physical(
    cursor: &mut Cursor<'_>,
    expected: ResourcePlan,
    cap: usize,
    header: &Header,
) -> Result<UnverifiedPhysical, CheckpointError> {
    PhysicalArchive::read(
        cursor.take(header.physical_size)?,
        expected,
        header.physical_size,
        cap,
    )
}
fn read_history(
    cursor: &mut Cursor<'_>,
    cap: usize,
    header: &Header,
) -> Result<RunHistory, CheckpointError> {
    history::read(
        cursor.take(header.history_size)?,
        header.history_size,
        header.configuration.limits.maximum_attempts,
        cap,
    )
}
fn validate_profile(
    physical: &UnverifiedPhysical,
    history: &RunHistory,
    header: &Header,
) -> Result<(), CheckpointError> {
    let controller = history.controller();
    let initial = TickClock::from_rest(controller.clock().exponent(), controller.clock().target())
        .map_err(CheckpointError::InvalidHistory)?;
    if same_configuration(header.configuration, controller.configuration())
        && header.initial_clock == initial
        && physical.state().clock() == controller.clock()
        && physical.state().accepted_steps() == controller.committed() as u128
        && physical.state().epoch().0 == controller.committed() as u128
        && history.records().len() == header.records
    {
        return Ok(());
    }
    Err(CheckpointError::InvalidHistory(SolverError::InvalidPayload))
}
fn read_work(
    cursor: &mut Cursor<'_>,
    expected: ResourcePlan,
    history: &RunHistory,
    header: &Header,
) -> Result<Vec<IntegrationWork>, CheckpointError> {
    let source = CyclicSine::new(expected.domain()).map_err(CheckpointError::InvalidHistory)?;
    let limits = source.limits().ok_or(CheckpointError::InvalidEncoding)?;
    let mut work = work_storage(header.configuration.limits.maximum_attempts)
        .map_err(CheckpointError::InvalidHistory)?;
    for record in 0..header.records {
        let item = read_work_item(cursor)?;
        let outcome = history
            .records()
            .get(record)
            .ok_or(CheckpointError::InvalidEncoding)?
            .outcome;
        validate_work_item(item, header.configuration, limits, outcome)?;
        work.push(item);
    }
    Ok(work)
}
fn read_work_item(cursor: &mut Cursor<'_>) -> Result<IntegrationWork, CheckpointError> {
    Ok(IntegrationWork {
        calls: size(cursor)?,
        work_units: size(cursor)?,
        scalar_transforms: size(cursor)?,
    })
}
fn validate_work_item(
    item: IntegrationWork,
    configuration: Configuration,
    limits: ForceLimits,
    outcome: Outcome,
) -> Result<(), CheckpointError> {
    let work = item
        .calls
        .checked_mul(limits.work_units)
        .ok_or(CheckpointError::ResourceLimit)?;
    let transforms = limits
        .scalar_transforms
        .checked_add(10)
        .and_then(|count| item.calls.checked_mul(count))
        .ok_or(CheckpointError::ResourceLimit)?;
    let calls = configuration.method.rhs_calls();
    let valid_calls = match outcome {
        Outcome::Committed(_)
        | Outcome::Rejected(_)
        | Outcome::Refused {
            indicators: Some(_),
            ..
        } => item.calls == calls,
        Outcome::Refused {
            indicators: None, ..
        } => item.calls <= calls,
    };
    if !valid_calls || item.work_units != work || item.scalar_transforms != transforms {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}
fn finish_import(
    cursor: Cursor<'_>,
    expected: ResourcePlan,
    history: &RunHistory,
    header: &Header,
    initial_samples: usize,
) -> Result<(), CheckpointError> {
    let committed = history
        .controller()
        .committed()
        .checked_add(initial_samples)
        .ok_or(CheckpointError::ResourceLimit)?;
    let measured_refusal = matches!(
        history.records().last().map(|record| record.outcome),
        Some(Outcome::Refused {
            indicators: Some(_),
            ..
        })
    );
    if !cursor.remaining.is_empty()
        || (header.observer_work.samples != committed
            && !(header.observer_work.samples == committed + 1 && measured_refusal))
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    BalanceObserver::validate_restored(expected, header.observer_samples, header.observer_work)
        .map_err(CheckpointError::InvalidHistory)
}
