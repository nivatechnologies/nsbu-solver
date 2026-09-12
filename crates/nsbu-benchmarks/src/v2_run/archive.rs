//! Bounded, versioned external archives for exact-v2 runs.
//!
//! The payload is self describing but never self authorizing: admission always rebuilds a
//! [`Plan`] from the caller supplied plan and imported runs are marked unverified.
use super::{AttemptWork, Origin, Plan, Run};
use crate::runtime_force::IntegrationMode;
use crate::smooth_observer::BalanceObserverWork;
use nsbu_solver::{
    checkpoint::{
        history,
        physical::{PhysicalArchive, UnverifiedPhysical},
        CheckpointError,
    },
    domain::TickClock,
    experiment::log::RunHistory,
    spectral::FftBackend,
    SolverError,
};
use sha2::{Digest, Sha256};

mod admission;
const MAGIC: &[u8; 8] = b"NSBUV2A1";
const VERSION: u16 = 1;
const HASH: usize = 32;
const HEADER: usize = 384;
const WORK: usize = 16 * 6;

/// Decoded external v2 data. Its origin is always [`Origin::ExternalUnverified`].
#[derive(Debug)]
pub struct ImportedV2Run {
    pub(super) physical: UnverifiedPhysical,
    pub(super) history: RunHistory,
    pub(super) work: Vec<AttemptWork>,
    pub(super) observer_work: BalanceObserverWork,
    pub(super) plan: Plan,
}

impl ImportedV2Run {
    /// External payloads are never promoted to an internally trusted origin.
    pub fn origin(&self) -> Origin {
        Origin::ExternalUnverified
    }
    /// Imported physical state, exposed read only.
    pub fn state(&self) -> &nsbu_solver::domain::SpectralState {
        self.physical.state()
    }
    /// Imported replayed history, exposed read only.
    pub fn history(&self) -> &RunHistory {
        &self.history
    }
    /// Imported integration ledger, exposed read only.
    pub fn work(&self) -> &[AttemptWork] {
        &self.work
    }
    /// Imported observer ledger.
    pub fn observer_work(&self) -> BalanceObserverWork {
        self.observer_work
    }
    /// Continue with freshly allocated numerical scratch under a bounded cap.
    pub fn continue_unverified(self, cap: usize) -> Result<Run, SolverError> {
        if self.plan.resources().total() > cap {
            return Err(SolverError::ResourceLimit);
        }
        Run::restore_parts(
            self.plan,
            self.physical.into_state(),
            self.history,
            self.work,
            self.observer_work,
            Origin::ExternalUnverified,
        )
    }
}

/// Exact encoded size including the SHA-256 trailer.
pub fn encoded_len(run: &Run) -> Result<usize, CheckpointError> {
    core_len(run)
}
/// Maximum encoded size admitted by a finite plan.
pub fn maximum_encoded_len(plan: Plan) -> Result<usize, CheckpointError> {
    require_direct(plan)?;
    let p = PhysicalArchive::encoded_len_for_plan(plan.resources())?;
    let h = history::maximum_encoded_len(plan.settings().configuration)?;
    let n = plan.settings().configuration.limits.maximum_attempts;
    HEADER
        .checked_add(p)
        .and_then(|x| x.checked_add(h))
        .and_then(|x| x.checked_add(n.checked_mul(WORK)?))
        .and_then(|x| x.checked_add(HASH))
        .ok_or(CheckpointError::ResourceLimit)
}

/// Peak storage required while decoding and preparing continuation.
pub fn read_reservation(plan: Plan, records: usize) -> Result<usize, CheckpointError> {
    require_direct(plan)?;
    if records > plan.settings().configuration.limits.maximum_attempts {
        return Err(CheckpointError::ResourceLimit);
    }
    let physical = UnverifiedPhysical::reservation(plan.resources())
        .map_err(|_| CheckpointError::ResourceLimit)?;
    let history = history::reservation(plan.settings().configuration, records)?;
    let ledger = super::work::reservation(plan.settings().configuration.limits.maximum_attempts)
        .map_err(|_| CheckpointError::ResourceLimit)?;
    plan.resources()
        .total()
        .checked_add(physical)
        .and_then(|n| n.checked_add(history))
        .and_then(|n| n.checked_add(ledger))
        .and_then(|n| n.checked_add(std::mem::size_of::<ImportedV2Run>()))
        .ok_or(CheckpointError::ResourceLimit)
}
fn core_len(run: &Run) -> Result<usize, CheckpointError> {
    require_direct(run.plan())?;
    let p = PhysicalArchive::encoded_len(run.state())?;
    let h = history::encoded_len(run.history())?;
    HEADER
        .checked_add(p)
        .and_then(|x| x.checked_add(h))
        .and_then(|x| x.checked_add(run.work().len().checked_mul(WORK)?))
        .and_then(|x| x.checked_add(HASH))
        .ok_or(CheckpointError::ResourceLimit)
}

/// Encode an internally coherent run, preserving unused output suffix bytes.
pub fn write(run: &Run, output: &mut [u8]) -> Result<usize, CheckpointError> {
    let need = core_len(run)?;
    if output.len() < need {
        return Err(CheckpointError::ResourceLimit);
    }
    let s = run.plan().settings();
    let mut p = 0;
    put(output, &mut p, MAGIC);
    put(output, &mut p, &VERSION.to_le_bytes());
    put(output, &mut p, crate::CASE_SHA256.as_bytes());
    put(
        output,
        &mut p,
        &[match s.force.workers {
            0 => 0,
            _ => 1,
        }],
    );
    for d in s.force.samples.dimensions() {
        put(output, &mut p, &(d as u128).to_le_bytes());
    }
    put(output, &mut p, &(s.force.workers as u128).to_le_bytes());
    clock(output, &mut p, s.initial_clock);
    put(
        output,
        &mut p,
        &(run.state().plan().domain().lengths()[0].to_bits()).to_le_bytes(),
    );
    put(output, &mut p, &s.advective_limit.to_bits().to_le_bytes());
    config(output, &mut p, s.configuration);
    let ps = PhysicalArchive::encoded_len(run.state())?;
    let hs = history::encoded_len(run.history())?;
    for x in [ps, hs, run.work().len()] {
        put(output, &mut p, &(x as u128).to_le_bytes());
    }
    let ow = run.observer_work();
    for x in [ow.samples, ow.work_units, ow.scalar_transforms] {
        put(output, &mut p, &(x as u128).to_le_bytes());
    }
    debug_assert_eq!(p, HEADER);
    p += PhysicalArchive::write(run.state(), &mut output[p..p + ps])?;
    p += history::write(run.history(), &mut output[p..p + hs])?;
    for w in run.work() {
        for x in [
            w.integration()[0],
            w.integration()[1],
            w.integration()[2],
            w.observation().samples,
            w.observation().work_units,
            w.observation().scalar_transforms,
        ] {
            put(output, &mut p, &(x as u128).to_le_bytes());
        }
    }
    let hash = Sha256::digest(&output[..p]);
    put(output, &mut p, &hash);
    debug_assert_eq!(p, need);
    Ok(p)
}

/// Decode after validating hash, identity, byte bound, and storage bound.
pub fn read(
    bytes: &[u8],
    expected: Plan,
    maximum_bytes: usize,
    storage_cap: usize,
) -> Result<ImportedV2Run, CheckpointError> {
    require_direct(expected)?;
    admission::read(bytes, expected, maximum_bytes, storage_cap)
}

fn require_direct(plan: Plan) -> Result<(), CheckpointError> {
    if plan.integration_mode() == IntegrationMode::Direct
        && plan.fft_backend() == FftBackend::OwnedRadix
    {
        Ok(())
    } else {
        Err(CheckpointError::InvalidEncoding)
    }
}

pub(super) struct Cursor<'a> {
    pub(super) remaining: &'a [u8],
}
impl<'a> Cursor<'a> {
    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], CheckpointError> {
        let (a, b) = self
            .remaining
            .split_at_checked(n)
            .ok_or(CheckpointError::InvalidEncoding)?;
        self.remaining = b;
        Ok(a)
    }
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], CheckpointError> {
        self.take(N)?
            .try_into()
            .map_err(|_| CheckpointError::InvalidEncoding)
    }
}
pub(super) fn put(o: &mut [u8], p: &mut usize, b: &[u8]) {
    let e = *p + b.len();
    o[*p..e].copy_from_slice(b);
    *p = e
}
pub(super) fn size(c: &mut Cursor<'_>) -> Result<usize, CheckpointError> {
    usize::try_from(u128::from_le_bytes(c.array()?)).map_err(|_| CheckpointError::ResourceLimit)
}
pub(super) fn clock(o: &mut [u8], p: &mut usize, v: TickClock) {
    put(o, p, &v.exponent().to_le_bytes());
    for x in [v.target(), v.elapsed(), v.remaining()] {
        put(o, p, &x.to_le_bytes())
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
pub(super) fn config(
    o: &mut [u8],
    p: &mut usize,
    v: nsbu_solver::experiment::control::Configuration,
) {
    put(
        o,
        p,
        &[match v.method {
            nsbu_solver::integrators::method::Method::CoxMatthews => 1,
            nsbu_solver::integrators::method::Method::HochbruckOstermann => 2,
        }],
    );
    for x in [
        v.limits.endpoint,
        v.limits.step_ticks,
        v.limits.maximum_attempts as u128,
    ] {
        put(o, p, &x.to_le_bytes())
    }
    for x in v
        .tolerances
        .absolute
        .into_iter()
        .chain(v.tolerances.relative)
    {
        put(o, p, &x.to_bits().to_le_bytes())
    }
}
pub(super) fn read_config(
    c: &mut Cursor<'_>,
) -> Result<nsbu_solver::experiment::control::Configuration, CheckpointError> {
    let method = match c.take(1)?[0] {
        1 => nsbu_solver::integrators::method::Method::CoxMatthews,
        2 => nsbu_solver::integrators::method::Method::HochbruckOstermann,
        _ => return Err(CheckpointError::InvalidEncoding),
    };
    let e = u128::from_le_bytes(c.array()?);
    let st = u128::from_le_bytes(c.array()?);
    let m = size(c)?;
    let a = [
        f64::from_bits(u64::from_le_bytes(c.array()?)),
        f64::from_bits(u64::from_le_bytes(c.array()?)),
    ];
    let r = [
        f64::from_bits(u64::from_le_bytes(c.array()?)),
        f64::from_bits(u64::from_le_bytes(c.array()?)),
    ];
    Ok(nsbu_solver::experiment::control::Configuration {
        method,
        limits: nsbu_solver::integrators::trajectory::RunLimits {
            endpoint: e,
            step_ticks: st,
            maximum_attempts: m,
        },
        tolerances: nsbu_solver::integrators::indicator::Tolerances {
            absolute: a,
            relative: r,
        },
    })
}
