//! Persistent bounded workers for exact-v2 physical sampling, followed by the original serial FFT.
//! The sampler preserves every spatial point and its original arithmetic; it never evolves state.
mod admission;
mod pool;
mod sampling;
#[cfg(test)]
mod tests;
mod worker;
use super::V2Force;
use crate::time::BenchmarkTime;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{FftBackend, FftCatalog},
    Complex64, SolverError,
};
/// Explicit parallel force provider with constructor-owned workers, buffers and configured stacks.
/// All complete plane results are collected before transforming any output coefficient.
/// A child failure terminates this provider; previously returned coefficients remain the caller's.
pub struct ParallelV2Force {
    inner: V2Force,
    pool: pool::Pool,
    limits: ForceLimits,
}
impl ParallelV2Force {
    /// Declare every numerical buffer, worker metadata, configured stack and thread allowance.
    /// Workers must be between one and min(128, sampled axial planes). No allocation occurs here.
    pub fn preflight(
        domain: Domain,
        samples: Layout,
        workers: usize,
    ) -> Result<ForceLimits, SolverError> {
        admission::limits(domain, samples, workers)
    }
    /// Refuse insufficient capacity before constructing any FFT workspace, buffer or thread.
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
        let serial = V2Force::preflight(domain, samples)?;
        Ok(Self {
            inner: V2Force::new(domain, samples, serial.storage_bytes)?,
            pool: pool::Pool::new(samples, workers)?,
            limits,
        })
    }
    /// Declare storage with immutable FFT plans owned by an enclosing execution catalog.
    pub fn preflight_with_catalog(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
    ) -> Result<ForceLimits, SolverError> {
        admission::limits_with_catalog(domain, samples, workers, catalog)
    }
    /// Workspace-only declaration for an enclosing execution-owned backend catalog.
    pub fn preflight_with_fft_backend(
        domain: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        admission::limits_with_fft_backend(domain, samples, workers, backend)
    }
    /// Construct persistent samplers while sharing the enclosing immutable FFT catalog.
    pub fn new_with_catalog(
        domain: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_catalog(domain, samples, workers, catalog)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let serial = V2Force::preflight_with_catalog(domain, samples, catalog)?;
        Ok(Self {
            inner: V2Force::new_with_catalog(domain, samples, catalog, serial.storage_bytes)?,
            pool: pool::Pool::new(samples, workers)?,
            limits,
        })
    }
    /// Same actual once-per-active-plane scalar iteration count as the serial sampler.
    pub fn last_root_iterations(&self) -> usize {
        self.inner.last_root_iterations()
    }
    /// A failed worker report cannot be retried through this provider's partly consumed scratch.
    pub fn is_terminated(&self) -> bool {
        self.pool.failed()
    }
}
impl PrescribedForce for ParallelV2Force {
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
            .any(|v| v.len() != self.inner.retained.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidClock)?;
        let iterations = self.pool.execute(time, &mut self.inner.physical)?;
        self.inner.transform(output)?;
        self.inner.last_root_iterations = iterations;
        Ok(ForceWork {
            work_units: self.inner.sampled.real_len() + iterations,
            scalar_transforms: 3,
        })
    }
}
