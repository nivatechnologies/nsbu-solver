//! Opt-in exact-v2 force reuse within one independently admitted integration attempt.
use super::{ForceSettings, RunForce};
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64, SolverError,
};

const SLOTS: usize = 5;
const MAXIMUM_CALLS: usize = 15;
const ALLOCATION_ALLOWANCE: usize = 15 * 64;

/// Detailed attempt-local cache consumption, separate from solver operator work.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AttemptCacheWork {
    /// Calls admitted to the wrapper, including a failed provider call.
    pub calls: usize,
    /// Calls that required the original exact-v2 provider.
    pub provider_evaluations: usize,
    /// Original-provider work, conservatively retained on provider failure.
    pub provider_work_units: usize,
    /// Original-provider transforms, conservatively retained on provider failure.
    pub provider_scalar_transforms: usize,
    /// Exact-clock lookup operations.
    pub lookups: usize,
    /// Exact-clock comparisons performed by those lookups.
    pub clock_comparisons: usize,
    /// Complex coefficient words copied into the RHS source field.
    pub coefficient_words_copied: usize,
    /// Completed calls served from an already published slot.
    pub hits: usize,
    /// Calls that attempted to fill an empty slot.
    pub misses: usize,
}

struct Slot {
    values: [Vec<Complex64>; 3],
    epoch: Option<u128>,
}

/// Five-slot cache for the five exact stage clocks in one full/two-half attempt.
///
/// This type only owns [`RunForce`], whose exact-v2 values depend on clock and immutable settings,
/// not on evolving velocity. Every new attempt invalidates all slots. It is not used by the
/// default v2 [`crate::v2_run::Run`] or archive-v1 profile.
pub struct AttemptForceCache {
    inner: RunForce,
    inner_limits: ForceLimits,
    limits: ForceLimits,
    length: usize,
    expected: [Option<TickClock>; SLOTS],
    slots: [Slot; SLOTS],
    epoch: u128,
    active: bool,
    remaining_calls: usize,
    work: AttemptCacheWork,
}

impl AttemptForceCache {
    /// Complete persistent storage and worst-case per-call aggregate work.
    pub fn preflight(domain: Domain, settings: ForceSettings) -> Result<ForceLimits, SolverError> {
        let inner = settings.limits(domain)?;
        let length = domain.layout().half_len();
        let words = length.checked_mul(3).ok_or(SolverError::SizeOverflow)?;
        let payload = words
            .checked_mul(SLOTS)
            .and_then(|n| n.checked_mul(std::mem::size_of::<Complex64>()))
            .ok_or(SolverError::SizeOverflow)?;
        let wrapper = std::mem::size_of::<Self>()
            .checked_sub(std::mem::size_of::<RunForce>())
            .ok_or(SolverError::SizeOverflow)?;
        Ok(ForceLimits {
            storage_bytes: inner
                .storage_bytes
                .checked_add(payload)
                .and_then(|n| n.checked_add(ALLOCATION_ALLOWANCE))
                .and_then(|n| n.checked_add(wrapper))
                .ok_or(SolverError::SizeOverflow)?,
            work_units: inner
                .work_units
                .checked_add(words)
                .and_then(|n| n.checked_add(SLOTS))
                .ok_or(SolverError::SizeOverflow)?,
            scalar_transforms: inner.scalar_transforms,
            remaining_divisor: inner.remaining_divisor,
        })
    }

    /// Allocate all five slots after complete preflight and cap admission.
    pub fn new(domain: Domain, settings: ForceSettings, cap: usize) -> Result<Self, SolverError> {
        let limits = Self::preflight(domain, settings)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let inner_limits = settings.limits(domain)?;
        let inner = settings.build(domain, inner_limits.storage_bytes)?;
        let length = domain.layout().half_len();
        Ok(Self {
            inner,
            inner_limits,
            limits,
            length,
            expected: [None; SLOTS],
            slots: [
                slot(length)?,
                slot(length)?,
                slot(length)?,
                slot(length)?,
                slot(length)?,
            ],
            epoch: 0,
            active: false,
            remaining_calls: 0,
            work: AttemptCacheWork::default(),
        })
    }

    /// Detailed consumption for the current attempt.
    pub fn work(&self) -> AttemptCacheWork {
        self.work
    }

    /// Monotone cache-attempt generation; slots from older generations never hit.
    pub fn epoch(&self) -> u128 {
        self.epoch
    }

    fn invalidate(&mut self) {
        self.active = false;
        self.remaining_calls = 0;
        self.expected = [None; SLOTS];
        for slot in &mut self.slots {
            slot.epoch = None;
        }
        self.work = AttemptCacheWork::default();
    }

    fn open(&mut self, clock: TickClock, ticks: u128) -> Result<(), SolverError> {
        let stages = clock.stages(ticks)?;
        let next = self
            .epoch
            .checked_add(1)
            .ok_or(SolverError::EpochExhausted)?;
        self.inner.begin_attempt(clock, ticks, self.inner_limits)?;
        self.expected = stages.map(Some);
        self.epoch = next;
        self.remaining_calls = MAXIMUM_CALLS;
        self.active = true;
        Ok(())
    }

    fn lookup(&mut self, clock: TickClock) -> Result<usize, SolverError> {
        if !self.active || self.remaining_calls == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.remaining_calls -= 1;
        self.work.calls += 1;
        self.work.lookups += 1;
        for (index, expected) in self.expected.iter().enumerate() {
            self.work.clock_comparisons += 1;
            if *expected == Some(clock) {
                return Ok(index);
            }
        }
        Err(SolverError::InvalidClock)
    }

    fn validate_output(&self, output: &[&mut [Complex64]; 3]) -> Result<(), SolverError> {
        if output.iter().all(|values| values.len() == self.length) {
            Ok(())
        } else {
            Err(SolverError::InvalidPayload)
        }
    }

    fn copy_slot(&mut self, index: usize, output: [&mut [Complex64]; 3]) -> ForceWork {
        for (target, source) in output.into_iter().zip(&self.slots[index].values) {
            target.copy_from_slice(source);
        }
        let words = 3 * self.length;
        self.work.coefficient_words_copied += words;
        ForceWork {
            work_units: words + index + 1,
            scalar_transforms: 0,
        }
    }
}

impl PrescribedForce for AttemptForceCache {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn begin_attempt(
        &mut self,
        clock: TickClock,
        ticks: u128,
        limit: ForceLimits,
    ) -> Result<(), SolverError> {
        self.invalidate();
        if limit != self.limits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.open(clock, ticks)
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limit != self.limits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.validate_output(&output)?;
        let index = self.lookup(clock)?;
        if self.slots[index].epoch == Some(self.epoch) {
            self.work.hits += 1;
            return Ok(self.copy_slot(index, output));
        }
        self.work.misses += 1;
        self.work.provider_evaluations += 1;
        self.work.provider_work_units += self.inner_limits.work_units;
        self.work.provider_scalar_transforms += self.inner_limits.scalar_transforms;
        let report = self.inner.evaluate(
            clock,
            self.inner_limits,
            self.slots[index].values.each_mut().map(Vec::as_mut_slice),
        )?;
        if report.work_units > self.inner_limits.work_units
            || report.scalar_transforms > self.inner_limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.work.provider_work_units -= self.inner_limits.work_units - report.work_units;
        self.work.provider_scalar_transforms -=
            self.inner_limits.scalar_transforms - report.scalar_transforms;
        let mut aggregate = self.copy_slot(index, output);
        aggregate.work_units = aggregate
            .work_units
            .checked_add(report.work_units)
            .ok_or(SolverError::SizeOverflow)?;
        aggregate.scalar_transforms = report.scalar_transforms;
        self.slots[index].epoch = Some(self.epoch);
        Ok(aggregate)
    }
}

fn slot(length: usize) -> Result<Slot, SolverError> {
    Ok(Slot {
        values: [values(length)?, values(length)?, values(length)?],
        epoch: None,
    })
}

fn values(length: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(length, Complex64::new(0.0, 0.0));
    Ok(values)
}
