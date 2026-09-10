//! Version-one bounded external payloads for the immutable smooth-run profile.
use super::{observation::Observation, IntegrationWork, Origin, OwnedRun, SmoothRun};
use crate::{
    smooth::CyclicSine,
    smooth_observer::{BalanceObserver, BalanceObserverWork},
};
use nsbu_solver::{
    checkpoint::{history, physical::PhysicalArchive, CheckpointError},
    domain::{Epoch, ResourcePlan, SpectralState, TickClock},
    experiment::{control::Configuration, log::RunHistory},
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs, transaction::CandidateState},
    SolverError,
};
use sha2::{Digest, Sha256};
pub(super) mod admission;
const MAGIC: &[u8; 8] = b"NSBUSR01";
const VERSION: u16 = 1;
const HEADER: usize = 264;
const HASH: usize = 32;
const WORK: usize = 48;
/// Externally decoded smooth-run data. Its origin is always diagnostic and unverified.
#[derive(Debug)]
pub struct ImportedSmoothRun {
    pub(super) physical: nsbu_solver::checkpoint::physical::UnverifiedPhysical,
    pub(super) history: RunHistory,
    pub(super) work: Vec<IntegrationWork>,
    pub(super) observer_work: BalanceObserverWork,
    pub(super) configuration: Configuration,
    pub(super) initial_clock: TickClock,
    pub(super) observer_samples: usize,
    pub(super) advective_limit: f64,
}
impl ImportedSmoothRun {
    /// Always ExternalUnverified, regardless of the encoded tag.
    pub fn origin(&self) -> Origin {
        Origin::ExternalUnverified
    }
    /// Read-only decoded physical state.
    pub fn state(&self) -> &SpectralState {
        self.physical.state()
    }
    /// Frozen decoded execution configuration.
    pub fn configuration(&self) -> Configuration {
        self.configuration
    }
    /// Initial rest clock encoded for this finite execution profile.
    pub fn initial_clock(&self) -> TickClock {
        self.initial_clock
    }
    /// Maximum independently measured accepted states declared by the profile.
    pub fn observer_samples(&self) -> usize {
        self.observer_samples
    }
    /// Exact binary64 bits of the immutable advective limit.
    pub fn advective_limit_bits(&self) -> u64 {
        self.advective_limit.to_bits()
    }
    /// Read-only replayed controller and balance history.
    pub fn history(&self) -> &RunHistory {
        &self.history
    }
    /// Continue as an externally unverified diagnostic run with fresh private scratch.
    pub fn continue_unverified(self, cap: usize) -> Result<SmoothRun, SolverError> {
        let work = self.observer_work;
        self.continue_observed::<BalanceObserver>(work, cap)
    }
    pub(super) fn continue_observed<O: Observation>(
        self,
        snapshot: O::Snapshot,
        cap: usize,
    ) -> Result<OwnedRun<O>, SolverError> {
        let plan = self.physical.state().plan();
        if plan.total() > cap {
            return Err(SolverError::ResourceLimit);
        }
        let state = self.physical.into_state();
        let source = CyclicSine::new(plan.domain())?;
        let candidate = CandidateState::new(plan, self.initial_clock, Epoch(0))?;
        let attempts = AttemptWorkspace::new_with_method(plan, self.configuration.method)?;
        let rhs = SpectralRhs::new(
            plan.domain(),
            source,
            self.advective_limit,
            plan.classes()[5],
        )?;
        let observer = O::restore(plan, self.observer_samples, &state, snapshot)?;
        Ok(OwnedRun {
            state,
            candidate,
            attempts,
            rhs,
            observer,
            history: self.history,
            work: self.work,
            configuration: self.configuration,
            initial_clock: self.initial_clock,
            observer_samples: self.observer_samples,
            advective_limit: self.advective_limit,
            origin: Origin::ExternalUnverified,
        })
    }
}
/// Exact bytes including a SHA-256 trailer, checked before writing caller storage.
pub fn encoded_len(run: &SmoothRun) -> Result<usize, CheckpointError> {
    core_len(run)
}
pub(super) fn core_len<O: Observation>(run: &OwnedRun<O>) -> Result<usize, CheckpointError> {
    let physical = PhysicalArchive::encoded_len(&run.state)?;
    let history = history::encoded_len(&run.history)?;
    run.work
        .len()
        .checked_mul(WORK)
        .and_then(|work| {
            HEADER
                .checked_add(physical)?
                .checked_add(history)?
                .checked_add(work)
        })
        .and_then(|size| size.checked_add(HASH))
        .ok_or(CheckpointError::ResourceLimit)
}
/// Largest archive for a finite admitted profile, without allocating a live run.
pub fn maximum_encoded_len(
    plan: ResourcePlan,
    configuration: Configuration,
) -> Result<usize, CheckpointError> {
    let physical = PhysicalArchive::encoded_len_for_plan(plan)?;
    let history = history::maximum_encoded_len(configuration)?;
    configuration
        .limits
        .maximum_attempts
        .checked_mul(WORK)
        .and_then(|work| {
            HEADER
                .checked_add(physical)?
                .checked_add(history)?
                .checked_add(work)
        })
        .and_then(|size| size.checked_add(HASH))
        .ok_or(CheckpointError::ResourceLimit)
}
/// Encode a coherent owner state. The tag records diagnostic origin but never qualifies it.
pub fn write(run: &SmoothRun, output: &mut [u8]) -> Result<usize, CheckpointError> {
    write_core(run, output)
}
pub(super) fn write_core<O: Observation>(
    run: &OwnedRun<O>,
    output: &mut [u8],
) -> Result<usize, CheckpointError> {
    let required = core_len(run)?;
    if output.len() < required {
        return Err(CheckpointError::ResourceLimit);
    }
    let physical = PhysicalArchive::encoded_len(&run.state)?;
    let history_size = history::encoded_len(&run.history)?;
    let mut p = 0;
    put(output, &mut p, MAGIC);
    put(output, &mut p, &VERSION.to_le_bytes());
    put(
        output,
        &mut p,
        &[match run.origin {
            Origin::InternalFromRest => 1,
            Origin::ExternalUnverified => 2,
        }],
    );
    clock(output, &mut p, run.initial_clock);
    configuration(output, &mut p, run.configuration);
    put(
        output,
        &mut p,
        &(run.observer_samples as u128).to_le_bytes(),
    );
    put(output, &mut p, &run.advective_limit.to_bits().to_le_bytes());
    put(output, &mut p, &(physical as u128).to_le_bytes());
    put(output, &mut p, &(history_size as u128).to_le_bytes());
    put(output, &mut p, &(run.work.len() as u128).to_le_bytes());
    for value in [
        run.observer.consumption().samples,
        run.observer.consumption().work_units,
        run.observer.consumption().scalar_transforms,
    ] {
        put(output, &mut p, &(value as u128).to_le_bytes());
    }
    debug_assert_eq!(p, HEADER);
    p += PhysicalArchive::write(&run.state, &mut output[p..p + physical])?;
    p += history::write(&run.history, &mut output[p..p + history_size])?;
    for work in &run.work {
        for value in [work.calls, work.work_units, work.scalar_transforms] {
            put(output, &mut p, &(value as u128).to_le_bytes());
        }
    }
    let hash = Sha256::digest(&output[..p]);
    put(output, &mut p, &hash);
    Ok(p)
}
/// Verify bounded external bytes against an expected plan and return an unqualified import.
pub fn read(
    bytes: &[u8],
    expected: ResourcePlan,
    maximum_bytes: usize,
    storage_cap: usize,
) -> Result<ImportedSmoothRun, CheckpointError> {
    admission::read(bytes, expected, maximum_bytes, storage_cap)
}
#[derive(Clone, Copy)]
pub(super) struct Header {
    pub(super) initial_clock: TickClock,
    pub(super) configuration: Configuration,
    pub(super) observer_samples: usize,
    pub(super) advective_limit: f64,
    pub(super) physical_size: usize,
    pub(super) history_size: usize,
    pub(super) records: usize,
    pub(super) observer_work: BalanceObserverWork,
}
pub(super) fn put(output: &mut [u8], p: &mut usize, bytes: &[u8]) {
    let end = *p + bytes.len();
    output[*p..end].copy_from_slice(bytes);
    *p = end;
}
pub(super) struct Cursor<'a> {
    pub(super) remaining: &'a [u8],
}
impl<'a> Cursor<'a> {
    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], CheckpointError> {
        let (head, tail) = self
            .remaining
            .split_at_checked(n)
            .ok_or(CheckpointError::InvalidEncoding)?;
        self.remaining = tail;
        Ok(head)
    }
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], CheckpointError> {
        let mut value = [0; N];
        value.copy_from_slice(self.take(N)?);
        Ok(value)
    }
}
pub(super) fn size(c: &mut Cursor<'_>) -> Result<usize, CheckpointError> {
    usize::try_from(u128::from_le_bytes(c.array()?)).map_err(|_| CheckpointError::ResourceLimit)
}
fn clock(output: &mut [u8], p: &mut usize, value: TickClock) {
    put(output, p, &value.exponent().to_le_bytes());
    for value in [value.target(), value.elapsed(), value.remaining()] {
        put(output, p, &value.to_le_bytes());
    }
}
pub(super) fn read_clock(c: &mut Cursor<'_>) -> Result<TickClock, CheckpointError> {
    TickClock::restore(
        i32::from_le_bytes(c.array()?),
        u128::from_le_bytes(c.array()?),
        u128::from_le_bytes(c.array()?),
        u128::from_le_bytes(c.array()?),
    )
    .map_err(CheckpointError::InvalidHistory)
}
pub(super) fn is_rest(clock: TickClock) -> bool {
    TickClock::from_rest(clock.exponent(), clock.target()).is_ok_and(|rest| rest == clock)
}
fn configuration(output: &mut [u8], p: &mut usize, value: Configuration) {
    put(
        output,
        p,
        &[match value.method {
            nsbu_solver::integrators::method::Method::CoxMatthews => 1,
            nsbu_solver::integrators::method::Method::HochbruckOstermann => 2,
        }],
    );
    for value in [
        value.limits.endpoint,
        value.limits.step_ticks,
        value.limits.maximum_attempts as u128,
    ] {
        put(output, p, &value.to_le_bytes());
    }
    for value in value
        .tolerances
        .absolute
        .into_iter()
        .chain(value.tolerances.relative)
    {
        put(output, p, &value.to_bits().to_le_bytes());
    }
}
pub(super) fn read_configuration(c: &mut Cursor<'_>) -> Result<Configuration, CheckpointError> {
    let method = match c.take(1)?[0] {
        1 => nsbu_solver::integrators::method::Method::CoxMatthews,
        2 => nsbu_solver::integrators::method::Method::HochbruckOstermann,
        _ => return Err(CheckpointError::InvalidEncoding),
    };
    let endpoint = u128::from_le_bytes(c.array()?);
    let step_ticks = u128::from_le_bytes(c.array()?);
    let maximum_attempts = size(c)?;
    let values = [
        f64::from_bits(u64::from_le_bytes(c.array()?)),
        f64::from_bits(u64::from_le_bytes(c.array()?)),
        f64::from_bits(u64::from_le_bytes(c.array()?)),
        f64::from_bits(u64::from_le_bytes(c.array()?)),
    ];
    Ok(Configuration {
        method,
        limits: nsbu_solver::integrators::trajectory::RunLimits {
            endpoint,
            step_ticks,
            maximum_attempts,
        },
        tolerances: nsbu_solver::integrators::indicator::Tolerances {
            absolute: [values[0], values[1]],
            relative: [values[2], values[3]],
        },
    })
}
