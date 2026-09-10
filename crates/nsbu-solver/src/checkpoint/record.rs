//! Fixed-width raw measured attempt records, preserving binary64 bits and exact tick clocks.
use super::{
    bytes::{put, Cursor},
    failure, CheckpointError,
};
use crate::{
    diagnostics::{balances::BalanceSample, norms::Norms},
    domain::TickClock,
    experiment::{control::Outcome, log::AttemptRecord},
    integrators::indicator::Indicators,
};
pub(super) const WIDTH: usize = 176;

pub(super) fn clock(output: &mut [u8], position: &mut usize, clock: TickClock) {
    put(output, position, &clock.exponent().to_le_bytes());
    for ticks in [clock.target(), clock.elapsed(), clock.remaining()] {
        put(output, position, &ticks.to_le_bytes());
    }
}
pub(super) fn read_clock(cursor: &mut Cursor<'_>) -> Result<TickClock, CheckpointError> {
    let exponent = i32::from_le_bytes(cursor.array()?);
    let target = u128::from_le_bytes(cursor.array()?);
    let elapsed = u128::from_le_bytes(cursor.array()?);
    let remaining = u128::from_le_bytes(cursor.array()?);
    TickClock::restore(exponent, target, elapsed, remaining)
        .map_err(CheckpointError::InvalidHistory)
}
pub(super) fn write(record: AttemptRecord, output: &mut [u8], position: &mut usize) {
    clock(output, position, record.start);
    let (tag, cause) = match record.outcome {
        Outcome::Committed(_) => (1, 0),
        Outcome::Rejected(_) => (2, 0),
        Outcome::Refused { cause, .. } => (3, failure::code(cause)),
    };
    let indicators = record.outcome.indicators();
    put(
        output,
        position,
        &[tag, cause, u8::from(indicators.is_some())],
    );
    let values = indicators.unwrap_or(Indicators {
        errors: [0.0; 2],
        ratios: [0.0; 2],
    });
    for value in values.errors.into_iter().chain(values.ratios) {
        put(output, position, &value.to_bits().to_le_bytes());
    }
    put(output, position, &[u8::from(record.sample.is_some())]);
    let sample = record.sample.unwrap_or(BalanceSample::REST);
    let norms = sample.norms;
    for value in [
        norms.l2,
        norms.h1,
        norms.vorticity_l2,
        norms.divergence_l2,
        sample.energy,
        sample.enstrophy,
        sample.energy_dissipation,
        sample.forcing_work,
        sample.stretching,
        sample.enstrophy_dissipation,
        sample.vorticity_forcing,
    ] {
        put(output, position, &value.to_bits().to_le_bytes());
    }
}
pub(super) fn read(cursor: &mut Cursor<'_>) -> Result<AttemptRecord, CheckpointError> {
    let start = read_clock(cursor)?;
    let tag = cursor.take(1)?[0];
    let cause = cursor.take(1)?[0];
    let present = cursor.take(1)?[0];
    let values = floats::<4>(cursor)?;
    let indicators = match present {
        1 => Some(Indicators {
            errors: [values[0], values[1]],
            ratios: [values[2], values[3]],
        }),
        0 if values.iter().all(|v| v.to_bits() == 0) => None,
        _ => return Err(CheckpointError::InvalidEncoding),
    };
    let outcome = outcome(tag, cause, indicators)?;
    let present = cursor.take(1)?[0];
    let values = floats::<11>(cursor)?;
    let sample = sample(present, values)?;
    Ok(AttemptRecord {
        start,
        outcome,
        sample,
    })
}
fn outcome(tag: u8, cause: u8, indicators: Option<Indicators>) -> Result<Outcome, CheckpointError> {
    match (tag, cause, indicators) {
        (1, 0, Some(values)) => Ok(Outcome::Committed(values)),
        (2, 0, Some(values)) => Ok(Outcome::Rejected(values)),
        (3, cause, indicators) => Ok(Outcome::Refused {
            cause: failure::decode(cause)?,
            indicators,
        }),
        _ => Err(CheckpointError::InvalidEncoding),
    }
}
fn sample(present: u8, values: [f64; 11]) -> Result<Option<BalanceSample>, CheckpointError> {
    match present {
        0 if values.iter().all(|v| v.to_bits() == 0) => Ok(None),
        1 => Ok(Some(BalanceSample {
            norms: Norms {
                l2: values[0],
                h1: values[1],
                vorticity_l2: values[2],
                divergence_l2: values[3],
            },
            energy: values[4],
            enstrophy: values[5],
            energy_dissipation: values[6],
            forcing_work: values[7],
            stretching: values[8],
            enstrophy_dissipation: values[9],
            vorticity_forcing: values[10],
        })),
        _ => Err(CheckpointError::InvalidEncoding),
    }
}
pub(super) fn floats<const N: usize>(cursor: &mut Cursor<'_>) -> Result<[f64; N], CheckpointError> {
    let mut values = [0.0; N];
    for value in &mut values {
        *value = f64::from_bits(u64::from_le_bytes(cursor.array()?));
    }
    Ok(values)
}
