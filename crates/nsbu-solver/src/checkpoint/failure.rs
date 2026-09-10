//! Stable version-one failure codes; enum layout and debug strings are never serialized.
use super::CheckpointError;
use crate::SolverError;

pub(super) fn code(error: SolverError) -> u8 {
    match error {
        SolverError::InvalidDomain => 1,
        SolverError::InvalidIndex => 2,
        SolverError::SizeOverflow => 3,
        SolverError::InvalidClock => 4,
        SolverError::InvalidStep => 5,
        SolverError::ClockCapacityExceeded => 6,
        SolverError::EpochExhausted => 7,
        SolverError::ResourceLimit => 8,
        SolverError::AllocationFailed => 9,
        SolverError::InvalidPayload => 10,
        SolverError::InvalidSpectrum => 11,
        SolverError::ArithmeticResolutionLimited => 12,
        SolverError::StaleAttempt => 13,
        SolverError::RetryLimit => 14,
        SolverError::UnknownProviderCost => 15,
        SolverError::ProviderBudgetExceeded => 16,
        SolverError::AdvectiveLimit => 17,
    }
}
pub(super) fn decode(code: u8) -> Result<SolverError, CheckpointError> {
    let error = match code {
        1 => SolverError::InvalidDomain,
        2 => SolverError::InvalidIndex,
        3 => SolverError::SizeOverflow,
        4 => SolverError::InvalidClock,
        5 => SolverError::InvalidStep,
        6 => SolverError::ClockCapacityExceeded,
        7 => SolverError::EpochExhausted,
        8 => SolverError::ResourceLimit,
        9 => SolverError::AllocationFailed,
        10 => SolverError::InvalidPayload,
        11 => SolverError::InvalidSpectrum,
        12 => SolverError::ArithmeticResolutionLimited,
        13 => SolverError::StaleAttempt,
        14 => SolverError::RetryLimit,
        15 => SolverError::UnknownProviderCost,
        16 => SolverError::ProviderBudgetExceeded,
        17 => SolverError::AdvectiveLimit,
        _ => return Err(CheckpointError::InvalidEncoding),
    };
    Ok(error)
}
