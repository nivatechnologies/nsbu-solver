//! Harness-local five-clock cache around the exact parallel-reduced provider.
use nsbu_benchmarks::provider::parallel_reduced::{
    ParallelReducedV2Force, ParallelReducedV2ForceW3,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{FftBackend, FftCatalog, ParallelFftIdentity, W3FftIdentity},
    Complex64, SolverError,
};

const SLOTS: usize = 5;
const CALLS: usize = 15;
const ALLOCATION_ALLOWANCE: usize = 15 * 64;

struct Slot {
    values: [Vec<Complex64>; 3],
    epoch: Option<u128>,
}

pub struct CachedReducedForce {
    inner: Inner,
    inner_limits: ForceLimits,
    limits: ForceLimits,
    expected: [Option<TickClock>; SLOTS],
    slots: [Slot; SLOTS],
    epoch: u128,
    remaining: usize,
    hits: usize,
    misses: usize,
}

enum Inner {
    Serial(ParallelReducedV2Force),
    W3(ParallelReducedV2ForceW3),
}

impl Inner {
    fn begin_attempt(
        &mut self,
        clock: TickClock,
        ticks: u128,
        limits: ForceLimits,
    ) -> Result<(), SolverError> {
        match self {
            Self::Serial(inner) => inner.begin_attempt(clock, ticks, limits),
            Self::W3(inner) => inner.begin_attempt(clock, ticks, limits),
        }
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limits: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        match self {
            Self::Serial(inner) => inner.evaluate(clock, limits, output),
            Self::W3(inner) => inner.evaluate(clock, limits, output),
        }
    }

    fn w3_identity(&self) -> Option<W3FftIdentity> {
        match self {
            Self::Serial(_) => None,
            Self::W3(inner) => Some(inner.w3_fft_identity()),
        }
    }
}

impl CachedReducedForce {
    pub fn preflight(
        domain: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
        w3: bool,
        fft_workers: Option<usize>,
    ) -> Result<ForceLimits, SolverError> {
        let inner = if let Some(fft_workers) = fft_workers {
            ParallelReducedV2ForceW3::preflight_with_parallel_fft_backend(
                domain,
                samples,
                workers,
                backend,
                fft_workers,
            )?
        } else if w3 {
            ParallelReducedV2ForceW3::preflight_with_fft_backend(domain, samples, workers, backend)?
        } else {
            ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, workers, backend)?
        };
        limits(inner, domain.layout().half_len())
    }

    pub fn new(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        cap: usize,
        w3: bool,
        fft_workers: Option<usize>,
    ) -> Result<Self, SolverError> {
        let inner_limits = if let Some(fft_workers) = fft_workers {
            ParallelReducedV2ForceW3::preflight_with_parallel_fft_backend(
                domain,
                samples,
                workers,
                catalog.backend(),
                fft_workers,
            )?
        } else if w3 {
            ParallelReducedV2ForceW3::preflight_with_fft_backend(
                domain,
                samples,
                workers,
                catalog.backend(),
            )?
        } else {
            ParallelReducedV2Force::preflight_with_fft_backend(
                domain,
                samples,
                workers,
                catalog.backend(),
            )?
        };
        let limits = limits(inner_limits, domain.layout().half_len())?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let inner = if let Some(fft_workers) = fft_workers {
            Inner::W3(ParallelReducedV2ForceW3::new_with_catalog_parallel_fft(
                domain,
                samples,
                workers,
                catalog,
                fft_workers,
                inner_limits.storage_bytes,
            )?)
        } else if w3 {
            Inner::W3(ParallelReducedV2ForceW3::new_with_catalog(
                domain,
                samples,
                workers,
                catalog,
                inner_limits.storage_bytes,
            )?)
        } else {
            Inner::Serial(ParallelReducedV2Force::new_with_catalog(
                domain,
                samples,
                workers,
                catalog,
                inner_limits.storage_bytes,
            )?)
        };
        let length = domain.layout().half_len();
        Ok(Self {
            inner,
            inner_limits,
            limits,
            expected: [None; SLOTS],
            slots: [
                slot(length)?,
                slot(length)?,
                slot(length)?,
                slot(length)?,
                slot(length)?,
            ],
            epoch: 0,
            remaining: 0,
            hits: 0,
            misses: 0,
        })
    }

    pub fn hit_miss(&self) -> [usize; 2] {
        [self.hits, self.misses]
    }

    pub fn w3_identity(&self) -> Option<W3FftIdentity> {
        self.inner.w3_identity()
    }

    pub fn parallel_fft_identity(&self) -> Option<ParallelFftIdentity> {
        match &self.inner {
            Inner::Serial(_) => None,
            Inner::W3(inner) => inner.parallel_fft_identity(),
        }
    }

    fn copy(&self, index: usize, output: [&mut [Complex64]; 3]) -> Result<ForceWork, SolverError> {
        for (target, source) in output.into_iter().zip(&self.slots[index].values) {
            if target.len() != source.len() {
                return Err(SolverError::InvalidPayload);
            }
            target.copy_from_slice(source);
        }
        let words = self.slots[index].values[0]
            .len()
            .checked_mul(3)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(ForceWork {
            work_units: words + index + 1,
            scalar_transforms: 0,
        })
    }
}

impl PrescribedForce for CachedReducedForce {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn begin_attempt(
        &mut self,
        clock: TickClock,
        ticks: u128,
        limit: ForceLimits,
    ) -> Result<(), SolverError> {
        if limit != self.limits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or(SolverError::EpochExhausted)?;
        self.expected = clock.stages(ticks)?.map(Some);
        self.remaining = CALLS;
        self.hits = 0;
        self.misses = 0;
        self.inner.begin_attempt(clock, ticks, self.inner_limits)
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limit != self.limits || self.remaining == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.remaining -= 1;
        let index = self
            .expected
            .iter()
            .position(|expected| *expected == Some(clock))
            .ok_or(SolverError::InvalidClock)?;
        if self.slots[index].epoch == Some(self.epoch) {
            self.hits += 1;
            return self.copy(index, output);
        }
        self.misses += 1;
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
        self.slots[index].epoch = Some(self.epoch);
        let mut aggregate = self.copy(index, output)?;
        aggregate.work_units = aggregate
            .work_units
            .checked_add(report.work_units)
            .ok_or(SolverError::SizeOverflow)?;
        aggregate.scalar_transforms = report.scalar_transforms;
        Ok(aggregate)
    }
}

fn limits(inner: ForceLimits, length: usize) -> Result<ForceLimits, SolverError> {
    let words = length.checked_mul(3).ok_or(SolverError::SizeOverflow)?;
    let payload = words
        .checked_mul(SLOTS)
        .and_then(|n| n.checked_mul(size_of::<Complex64>()))
        .ok_or(SolverError::SizeOverflow)?;
    let wrapper = size_of::<CachedReducedForce>()
        .checked_sub(size_of::<Inner>())
        .ok_or(SolverError::SizeOverflow)?;
    Ok(ForceLimits {
        storage_bytes: inner
            .storage_bytes
            .checked_add(payload)
            .and_then(|n| n.checked_add(ALLOCATION_ALLOWANCE + wrapper))
            .ok_or(SolverError::SizeOverflow)?,
        work_units: inner
            .work_units
            .checked_add(words + SLOTS)
            .ok_or(SolverError::SizeOverflow)?,
        scalar_transforms: inner.scalar_transforms,
        remaining_divisor: inner.remaining_divisor,
    })
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
