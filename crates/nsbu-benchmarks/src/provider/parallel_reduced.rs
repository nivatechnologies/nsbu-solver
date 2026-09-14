//! Persistent plane-parallel sampling with the explicit reduced exact-v2 arithmetic.
//!
//! This optional provider retains the reduced evaluator's pointwise operation order and the
//! serial reduced provider's FFT/transfer order. Worker scheduling changes neither order.
use super::{
    parallel::{admission, pool::Pool, sampling::Arithmetic},
    reduced::{ReducedV2Force, ReducedV2ForceW3},
};
use crate::time::BenchmarkTime;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{FftBackend, FftCatalog, ParallelFftIdentity, W3FftIdentity},
    Complex64, SolverError,
};

/// Construction identity for an experimental parallel reduced-force owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelReducedIdentity {
    /// Retained output layout.
    pub retained: Layout,
    /// Physical sampling and FFT layout.
    pub sampled: Layout,
    /// Persistent plane-worker count.
    pub workers: usize,
    /// Capacity supplied to the constructor.
    pub cap_bytes: usize,
}

/// Reduced exact-v2 force values sampled by a fixed persistent plane pool.
pub struct ParallelReducedV2Force {
    inner: ReducedV2Force,
    pool: Pool,
    limits: ForceLimits,
    identity: ParallelReducedIdentity,
    fft_backend: FftBackend,
}

impl ParallelReducedV2Force {
    /// Declare serial FFT storage, worker plane buffers, reduced roots and thread allowances.
    pub fn preflight(
        domain: Domain,
        samples: Layout,
        workers: usize,
    ) -> Result<ForceLimits, SolverError> {
        admission::reduced_limits(domain, samples, workers)
    }

    /// Construct all buffers and acknowledge every worker before returning.
    pub fn new(
        domain: Domain,
        samples: Layout,
        workers: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight(domain, samples, workers)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let serial = ReducedV2Force::preflight(domain, samples)?;
        Ok(Self {
            inner: ReducedV2Force::new(domain, samples, serial.storage_bytes)?,
            pool: Pool::new(samples, workers, Arithmetic::Reduced)?,
            limits,
            identity: ParallelReducedIdentity {
                retained: domain.layout(),
                sampled: samples,
                workers,
                cap_bytes: cap,
            },
            fft_backend: FftBackend::OwnedRadix,
        })
    }

    /// Declare all mutable storage when immutable FFT plans are execution-owned.
    pub fn preflight_with_fft_backend(
        domain: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        admission::reduced_limits_with_fft_backend(domain, samples, workers, backend)
    }

    /// Construct the composite parallel-reduced sampler with an explicit shared FFT catalog.
    pub fn new_with_catalog(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_fft_backend(domain, samples, workers, catalog.backend())?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let serial =
            ReducedV2Force::preflight_with_fft_backend(domain, samples, catalog.backend())?;
        Ok(Self {
            inner: ReducedV2Force::new_with_catalog(
                domain,
                samples,
                catalog,
                serial.storage_bytes,
            )?,
            pool: Pool::new(samples, workers, Arithmetic::Reduced)?,
            limits,
            identity: ParallelReducedIdentity {
                retained: domain.layout(),
                sampled: samples,
                workers,
                cap_bytes: cap,
            },
            fft_backend: catalog.backend(),
        })
    }

    /// Immutable scalar-transform backend used after physical sampling.
    pub fn fft_backend(&self) -> FftBackend {
        self.fft_backend
    }

    /// Exact retained/sample layouts, persistent worker count and supplied construction cap.
    pub fn identity(&self) -> ParallelReducedIdentity {
        self.identity
    }

    /// Scalar root iterations used by the latest successful request.
    pub fn last_root_iterations(&self) -> usize {
        self.inner.last_root_iterations()
    }

    /// A worker error permanently terminates this owner.
    pub fn is_terminated(&self) -> bool {
        self.pool.failed()
    }
}

impl PrescribedForce for ParallelReducedV2Force {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limits: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limits != self.limits || self.pool.failed() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        if output
            .iter()
            .any(|values| values.len() != self.identity.retained.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidClock)?;
        let iterations = self.pool.execute(time, &mut self.inner.physical)?;
        self.inner.transform(output)?;
        self.inner.last_root_iterations = iterations;
        Ok(ForceWork {
            work_units: self.identity.sampled.real_len() + iterations,
            scalar_transforms: 3,
        })
    }
}

/// Opt-in parallel sampler with an independently owned forward-only W3 FFT pool.
pub struct ParallelReducedV2ForceW3 {
    inner: ReducedV2ForceW3,
    pool: Pool,
    limits: ForceLimits,
    identity: ParallelReducedIdentity,
    fft_backend: FftBackend,
}

impl ParallelReducedV2ForceW3 {
    /// Complete provider reservation including its separate W3 FFT owner.
    pub fn preflight_with_fft_backend(
        domain: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        admission::reduced_w3_limits_with_fft_backend(domain, samples, workers, backend)
    }

    /// Complete provider reservation with one shared bounded intra-transform executor.
    pub fn preflight_with_parallel_fft_backend(
        domain: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
        fft_workers: usize,
    ) -> Result<ForceLimits, SolverError> {
        admission::reduced_parallel_w3_limits_with_fft_backend(
            domain,
            samples,
            workers,
            backend,
            fft_workers,
        )
    }

    /// Construct only after the complete sampling and W3 reservation fits.
    pub fn new_with_catalog(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_fft_backend(domain, samples, workers, catalog.backend())?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let transform_limits =
            ReducedV2ForceW3::preflight_with_fft_backend(domain, samples, catalog.backend())?;
        Ok(Self {
            inner: ReducedV2ForceW3::new_with_catalog(
                domain,
                samples,
                catalog,
                transform_limits.storage_bytes,
            )?,
            pool: Pool::new(samples, workers, Arithmetic::Reduced)?,
            limits,
            identity: ParallelReducedIdentity {
                retained: domain.layout(),
                sampled: samples,
                workers,
                cap_bytes: cap,
            },
            fft_backend: catalog.backend(),
        })
    }

    /// Construct the sampling pool and explicit shared-executor W3 transform.
    pub fn new_with_catalog_parallel_fft(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        fft_workers: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_parallel_fft_backend(
            domain,
            samples,
            workers,
            catalog.backend(),
            fft_workers,
        )?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let transform_limits = ReducedV2ForceW3::preflight_with_parallel_fft_backend(
            domain,
            samples,
            catalog.backend(),
            fft_workers,
        )?;
        Ok(Self {
            inner: ReducedV2ForceW3::new_with_catalog_parallel_fft(
                domain,
                samples,
                catalog,
                fft_workers,
                transform_limits.storage_bytes,
            )?,
            pool: Pool::new(samples, workers, Arithmetic::Reduced)?,
            limits,
            identity: ParallelReducedIdentity {
                retained: domain.layout(),
                sampled: samples,
                workers,
                cap_bytes: cap,
            },
            fft_backend: catalog.backend(),
        })
    }

    /// Existing sampling identity remains separate from the W3 execution identity.
    pub fn identity(&self) -> ParallelReducedIdentity {
        self.identity
    }

    /// Explicit transform-owner identity for artifact binding.
    pub fn w3_fft_identity(&self) -> W3FftIdentity {
        self.inner.fft_identity()
    }

    /// Shared intra-transform executor identity for the explicit parallel constructor.
    pub fn parallel_fft_identity(&self) -> Option<ParallelFftIdentity> {
        self.inner.parallel_fft_identity()
    }

    /// Immutable scalar-transform backend used after sampling.
    pub fn fft_backend(&self) -> FftBackend {
        self.fft_backend
    }

    /// A sampling-worker or W3 transform failure permanently terminates this provider.
    pub fn is_terminated(&self) -> bool {
        self.pool.failed() || self.inner.is_terminated()
    }
}

impl PrescribedForce for ParallelReducedV2ForceW3 {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limits: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limits != self.limits || self.is_terminated() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        if output
            .iter()
            .any(|values| values.len() != self.identity.retained.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidClock)?;
        let iterations = self.pool.execute(time, &mut self.inner.physical)?;
        self.inner.transform(output)?;
        self.inner.last_root_iterations = iterations;
        Ok(ForceWork {
            work_units: self.identity.sampled.real_len() + iterations,
            scalar_transforms: 3,
        })
    }
}

#[cfg(test)]
mod tests;
