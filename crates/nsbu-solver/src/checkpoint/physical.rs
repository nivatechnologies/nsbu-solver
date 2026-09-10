//! Exact, bounded physical-state payloads; qualification needs separately restored records.
use super::{
    bytes::{put, Cursor},
    CheckpointError,
};
use crate::{
    domain::{validate_spectrum, Epoch, ResourcePlan, SpectralState, TickClock},
    storage::field,
    Complex64, SolverError,
};

const MAGIC: &[u8; 8] = b"NSBUPH01";
const VERSION: u16 = 1;
const HEADER_BYTES: usize = 350;
const SPECTRUM_TOLERANCE: f64 = 1e-12;

/// Independently owned decoded physical storage without a restart qualification claim.
///
/// A caller must separately restore and verify controller, history, configuration, artifacts,
/// and provenance before deciding whether this state may continue a run.
#[derive(Debug)]
pub struct UnverifiedPhysical {
    state: SpectralState,
}

impl UnverifiedPhysical {
    /// Extra storage required for the owned Fourier fields and inline state metadata.
    pub fn reservation(plan: ResourcePlan) -> Result<usize, SolverError> {
        plan.domain()
            .layout()
            .half_len()
            .checked_mul(3 * std::mem::size_of::<Complex64>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Read-only decoded state; this type intentionally grants no continuation status.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }

    /// Transfer the independently allocated payload without allocation or coefficient changes.
    pub fn into_state(self) -> SpectralState {
        self.state
    }
}

/// Canonical version-one physical payload encoding and decoding.
pub struct PhysicalArchive;

impl PhysicalArchive {
    /// Exact physical encoding length for an admitted plan, without live state allocation.
    pub fn encoded_len_for_plan(plan: ResourcePlan) -> Result<usize, CheckpointError> {
        coefficient_bytes(plan.domain().layout().half_len())
    }

    /// Exact output length, checked before a caller provides writable output storage.
    pub fn encoded_len(state: &SpectralState) -> Result<usize, CheckpointError> {
        Self::encoded_len_for_plan(state.plan())
    }

    /// Encode all immutable plan identity and live physical bits into caller-owned storage.
    /// A short output is rejected before it is changed.
    pub fn write(state: &SpectralState, output: &mut [u8]) -> Result<usize, CheckpointError> {
        let required = Self::encoded_len(state)?;
        if output.len() < required {
            return Err(CheckpointError::ResourceLimit);
        }
        let plan = state.plan;
        let domain = plan.domain();
        let clock = state.clock;
        let mut position = 0;
        put(output, &mut position, MAGIC);
        put(output, &mut position, &VERSION.to_le_bytes());
        for dimension in domain.layout().dimensions() {
            put(output, &mut position, &(dimension as u128).to_le_bytes());
        }
        for length in domain.lengths() {
            put(output, &mut position, &length.to_bits().to_le_bytes());
        }
        put(
            output,
            &mut position,
            &domain.viscosity().to_bits().to_le_bytes(),
        );
        put(output, &mut position, &plan.epoch().0.to_le_bytes());
        for class in plan.classes() {
            put(output, &mut position, &(class as u128).to_le_bytes());
        }
        put(output, &mut position, &(plan.total() as u128).to_le_bytes());
        put(output, &mut position, &clock.exponent().to_le_bytes());
        put(output, &mut position, &clock.target().to_le_bytes());
        put(output, &mut position, &clock.elapsed().to_le_bytes());
        put(output, &mut position, &clock.remaining().to_le_bytes());
        put(output, &mut position, &state.epoch.0.to_le_bytes());
        put(output, &mut position, &state.accepted_steps.to_le_bytes());
        put(
            output,
            &mut position,
            &(domain.layout().half_len() as u128).to_le_bytes(),
        );
        for component in &state.components {
            for value in component {
                put(output, &mut position, &value.re.to_bits().to_le_bytes());
                put(output, &mut position, &value.im.to_bits().to_le_bytes());
            }
        }
        Ok(position)
    }

    /// Decode only a payload matching the separately approved resource plan and both caps.
    /// All byte counts and identity fields are checked before allocating Fourier storage.
    /// Conjugacy uses the integration profile's absolute componentwise tolerance of 1e-12.
    /// Nyquist coefficients must still be exactly zero; every imported bit is preserved.
    pub fn read(
        bytes: &[u8],
        expected: ResourcePlan,
        maximum_bytes: usize,
        maximum_storage: usize,
    ) -> Result<UnverifiedPhysical, CheckpointError> {
        if bytes.len() > maximum_bytes {
            return Err(CheckpointError::ResourceLimit);
        }
        if UnverifiedPhysical::reservation(expected).map_err(|_| CheckpointError::ResourceLimit)?
            > maximum_storage
        {
            return Err(CheckpointError::ResourceLimit);
        }
        let mut cursor = Cursor { remaining: bytes };
        if cursor.take(8)? != MAGIC || u16::from_le_bytes(cursor.array()?) != VERSION {
            return Err(CheckpointError::InvalidEncoding);
        }
        validate_plan(&mut cursor, expected)?;
        let (clock, epoch, accepted_steps, count) = read_state_header(&mut cursor, expected)?;
        let components = read_components(&mut cursor, expected, count)?;
        Ok(UnverifiedPhysical {
            state: SpectralState {
                plan: expected,
                clock,
                epoch,
                components,
                accepted_steps,
            },
        })
    }
}

fn read_state_header(
    cursor: &mut Cursor<'_>,
    expected: ResourcePlan,
) -> Result<(TickClock, Epoch, u128, usize), CheckpointError> {
    let exponent = i32::from_le_bytes(cursor.array()?);
    let target = u128::from_le_bytes(cursor.array()?);
    let elapsed = u128::from_le_bytes(cursor.array()?);
    let remaining = u128::from_le_bytes(cursor.array()?);
    let clock = TickClock::restore(exponent, target, elapsed, remaining)
        .map_err(|_| CheckpointError::InvalidEncoding)?;
    let epoch = Epoch(u128::from_le_bytes(cursor.array()?));
    let accepted_steps = u128::from_le_bytes(cursor.array()?);
    let count = usize::try_from(u128::from_le_bytes(cursor.array()?))
        .map_err(|_| CheckpointError::ResourceLimit)?;
    let expected_count = expected.domain().layout().half_len();
    if count != expected_count || cursor.remaining.len() != coefficient_bytes(count)? - HEADER_BYTES
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok((clock, epoch, accepted_steps, count))
}

fn read_components(
    cursor: &mut Cursor<'_>,
    expected: ResourcePlan,
    count: usize,
) -> Result<[Vec<Complex64>; 3], CheckpointError> {
    let mut components = field(count).map_err(|_| CheckpointError::ResourceLimit)?;
    for component in &mut components {
        for value in &mut *component {
            value.re = f64::from_bits(u64::from_le_bytes(cursor.array()?));
            value.im = f64::from_bits(u64::from_le_bytes(cursor.array()?));
        }
        // Accepted attempt fields use this same checked componentwise Hermitian tolerance.
        // Decoding never repairs or projects a payload that exceeds it.
        validate_spectrum(expected.domain().layout(), component, SPECTRUM_TOLERANCE)
            .map_err(|_| CheckpointError::InvalidEncoding)?;
    }
    Ok(components)
}

fn coefficient_bytes(count: usize) -> Result<usize, CheckpointError> {
    count
        .checked_mul(3 * std::mem::size_of::<Complex64>())
        .and_then(|bytes| HEADER_BYTES.checked_add(bytes))
        .ok_or(CheckpointError::ResourceLimit)
}

fn validate_plan(cursor: &mut Cursor<'_>, expected: ResourcePlan) -> Result<(), CheckpointError> {
    let domain = expected.domain();
    for dimension in domain.layout().dimensions() {
        if u128::from_le_bytes(cursor.array()?) != dimension as u128 {
            return Err(CheckpointError::InvalidEncoding);
        }
    }
    for length in domain.lengths() {
        if u64::from_le_bytes(cursor.array()?) != length.to_bits() {
            return Err(CheckpointError::InvalidEncoding);
        }
    }
    if u64::from_le_bytes(cursor.array()?) != domain.viscosity().to_bits()
        || u128::from_le_bytes(cursor.array()?) != expected.epoch().0
    {
        return Err(CheckpointError::InvalidEncoding);
    }
    for class in expected.classes() {
        if u128::from_le_bytes(cursor.array()?) != class as u128 {
            return Err(CheckpointError::InvalidEncoding);
        }
    }
    if u128::from_le_bytes(cursor.array()?) != expected.total() as u128 {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok(())
}
