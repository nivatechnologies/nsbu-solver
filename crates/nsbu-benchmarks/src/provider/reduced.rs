//! Optional serial sampled provider using the reduced degree-three force evaluator.
use crate::{
    reduced_force::axial::{self, AxialRoot},
    time::BenchmarkTime,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{
        transfer, FftBackend, FftCatalog, FftPlan, FftWorkspace, W3FftIdentity, W3FftMode,
        W3FftPool,
    },
    Complex64, SolverError,
};

#[derive(Debug)]
/// Serial sampled provider using reduced degree-three force arithmetic.
pub struct ReducedV2Force {
    retained: Layout,
    sampled: Layout,
    plan: FftPlan,
    workspace: FftWorkspace,
    pub(super) physical: [Vec<f64>; 3],
    spectral: Vec<Complex64>,
    roots: Vec<Option<AxialRoot>>,
    limits: ForceLimits,
    pub(super) last_root_iterations: usize,
}

impl ReducedV2Force {
    /// Declare complete provider storage and bounded per-request work.
    pub fn preflight(domain: Domain, sampled: Layout) -> Result<ForceLimits, SolverError> {
        Self::preflight_parts(domain, sampled, || FftPlan::reservation(sampled))
    }

    pub(crate) fn preflight_with_fft_backend(
        domain: Domain,
        sampled: Layout,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        Self::preflight_parts(domain, sampled, || {
            FftPlan::reservation_with_shared_backend(sampled, backend)
        })
    }

    fn preflight_parts(
        domain: Domain,
        sampled: Layout,
        fft: impl FnOnce() -> Result<usize, SolverError>,
    ) -> Result<ForceLimits, SolverError> {
        validate(domain, sampled)?;
        let buffers = sampled
            .real_len()
            .checked_mul(24)
            .and_then(|n| {
                sampled
                    .half_len()
                    .checked_mul(16)
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or(SolverError::SizeOverflow)?;
        let roots = sampled.dimensions()[2]
            .checked_mul(std::mem::size_of::<Option<AxialRoot>>())
            .ok_or(SolverError::SizeOverflow)?;
        // Backend-specific FFT admission follows generic provider overflow checks.
        let storage_bytes = fft()?
            .checked_add(buffers)
            .and_then(|n| n.checked_add(roots))
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>() + 12 * 64))
            .ok_or(SolverError::SizeOverflow)?;
        let work_units = sampled
            .real_len()
            .checked_mul(129)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(ForceLimits {
            storage_bytes,
            work_units,
            scalar_transforms: 3,
            remaining_divisor: 20,
        })
    }

    /// Construct the provider after admitting its complete storage reservation.
    pub fn new(domain: Domain, sampled: Layout, cap: usize) -> Result<Self, SolverError> {
        let limits = Self::preflight(domain, sampled)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let (plan, workspace) = FftPlan::new(sampled, cap)?;
        Self::allocate(domain, sampled, limits, plan, workspace)
    }

    pub(crate) fn new_with_catalog(
        domain: Domain,
        sampled: Layout,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_fft_backend(domain, sampled, catalog.backend())?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let (plan, workspace) = FftPlan::new_from_catalog(sampled, catalog, cap)?;
        Self::allocate(domain, sampled, limits, plan, workspace)
    }

    fn allocate(
        domain: Domain,
        sampled: Layout,
        limits: ForceLimits,
        plan: FftPlan,
        workspace: FftWorkspace,
    ) -> Result<Self, SolverError> {
        Ok(Self {
            retained: domain.layout(),
            sampled,
            plan,
            workspace,
            physical: [
                buffer(sampled.real_len(), 0.0)?,
                buffer(sampled.real_len(), 0.0)?,
                buffer(sampled.real_len(), 0.0)?,
            ],
            spectral: buffer(sampled.half_len(), Complex64::new(0.0, 0.0))?,
            roots: buffer(sampled.dimensions()[2], None)?,
            limits,
            last_root_iterations: 0,
        })
    }

    /// Return scalar root iterations used by the latest successful request.
    pub fn last_root_iterations(&self) -> usize {
        self.last_root_iterations
    }

    fn sample(&mut self, time: BenchmarkTime) -> Result<usize, SolverError> {
        let [nx, ny, nz] = self.sampled.dimensions();
        let mut iterations = 0;
        for (k, slot) in self.roots.iter_mut().enumerate() {
            *slot = AxialRoot::new(k as f64 / nz as f64, time)
                .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
            iterations += slot.map_or(0, AxialRoot::iterations);
        }
        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let point = [
                        i as f64 / nx as f64,
                        j as f64 / ny as f64,
                        k as f64 / nz as f64,
                    ];
                    let value = axial::evaluate(point, time, self.roots[k].as_ref())
                        .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
                    let index = (i * ny + j) * nz + k;
                    for (values, force) in self.physical.iter_mut().zip(value.force) {
                        values[index] = force;
                    }
                }
            }
        }
        Ok(iterations)
    }

    pub(super) fn transform(&mut self, output: [&mut [Complex64]; 3]) -> Result<(), SolverError> {
        for (physical, coefficients) in self.physical.iter().zip(output) {
            self.plan
                .forward(physical, &mut self.spectral, &mut self.workspace)?;
            transfer(self.sampled, self.retained, &self.spectral, coefficients)?;
        }
        Ok(())
    }
}

/// Reduced provider transform state for the explicit forward-only W3 path.
pub(super) struct ReducedV2ForceW3 {
    retained: Layout,
    sampled: Layout,
    transform: W3FftPool,
    pub(super) physical: [Vec<f64>; 3],
    _roots: Vec<Option<AxialRoot>>,
    _limits: ForceLimits,
    pub(super) last_root_iterations: usize,
}

impl ReducedV2ForceW3 {
    pub(super) fn preflight_with_fft_backend(
        domain: Domain,
        sampled: Layout,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        let mut limits = ReducedV2Force::preflight_with_fft_backend(domain, sampled, backend)?;
        let additional =
            W3FftPool::additional_reservation_with_backend(sampled, backend, W3FftMode::Forward)?;
        limits.storage_bytes = limits
            .storage_bytes
            .checked_add(additional)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(limits)
    }

    pub(super) fn preflight_with_parallel_fft_backend(
        domain: Domain,
        sampled: Layout,
        backend: FftBackend,
        fft_workers: usize,
    ) -> Result<ForceLimits, SolverError> {
        let mut limits = ReducedV2Force::preflight_with_fft_backend(domain, sampled, backend)?;
        let additional = W3FftPool::additional_parallel_reservation_with_backend(
            sampled,
            backend,
            W3FftMode::Forward,
            fft_workers,
        )?;
        limits.storage_bytes = limits
            .storage_bytes
            .checked_add(additional)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(limits)
    }

    pub(super) fn new_with_catalog(
        domain: Domain,
        sampled: Layout,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_fft_backend(domain, sampled, catalog.backend())?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let base_limits =
            ReducedV2Force::preflight_with_fft_backend(domain, sampled, catalog.backend())?;
        let serial =
            ReducedV2Force::new_with_catalog(domain, sampled, catalog, base_limits.storage_bytes)?;
        let ReducedV2Force {
            retained,
            sampled,
            plan,
            workspace,
            physical,
            spectral,
            roots,
            last_root_iterations,
            ..
        } = serial;
        let additional = limits
            .storage_bytes
            .checked_sub(base_limits.storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(Self {
            retained,
            sampled,
            transform: W3FftPool::from_scalar_lane(
                sampled,
                catalog,
                W3FftMode::Forward,
                (plan, workspace, spectral),
                additional,
            )?,
            physical,
            _roots: roots,
            _limits: limits,
            last_root_iterations,
        })
    }

    pub(super) fn new_with_catalog_parallel_fft(
        domain: Domain,
        sampled: Layout,
        catalog: &FftCatalog,
        fft_workers: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_parallel_fft_backend(
            domain,
            sampled,
            catalog.backend(),
            fft_workers,
        )?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let base_limits =
            ReducedV2Force::preflight_with_fft_backend(domain, sampled, catalog.backend())?;
        let serial =
            ReducedV2Force::new_with_catalog(domain, sampled, catalog, base_limits.storage_bytes)?;
        let ReducedV2Force {
            retained,
            sampled,
            plan,
            workspace,
            physical,
            spectral,
            roots,
            last_root_iterations,
            ..
        } = serial;
        let additional = limits
            .storage_bytes
            .checked_sub(base_limits.storage_bytes)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(Self {
            retained,
            sampled,
            transform: W3FftPool::from_scalar_lane_parallel(
                sampled,
                catalog,
                W3FftMode::Forward,
                fft_workers,
                (plan, workspace, spectral),
                additional,
            )?,
            physical,
            _roots: roots,
            _limits: limits,
            last_root_iterations,
        })
    }

    pub(super) fn transform(&mut self, output: [&mut [Complex64]; 3]) -> Result<(), SolverError> {
        if output.iter().any(|v| v.len() != self.retained.half_len()) {
            return Err(SolverError::InvalidPayload);
        }
        self.transform.forward3(&mut self.physical)?;
        for (axis, coefficients) in output.into_iter().enumerate() {
            self.transform.with_spectrum(axis, |spectrum| {
                transfer(self.sampled, self.retained, spectrum, coefficients)
            })??;
        }
        Ok(())
    }

    pub(super) fn fft_identity(&self) -> W3FftIdentity {
        self.transform.identity()
    }

    pub(super) fn parallel_fft_identity(
        &self,
    ) -> Option<nsbu_solver::spectral::ParallelFftIdentity> {
        self.transform.parallel_fft_identity()
    }

    pub(super) fn is_terminated(&self) -> bool {
        self.transform.is_terminated()
    }
}

impl PrescribedForce for ReducedV2Force {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
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
        if output.iter().any(|v| v.len() != self.retained.half_len()) {
            return Err(SolverError::InvalidPayload);
        }
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidClock)?;
        let iterations = self.sample(time)?;
        self.transform(output)?;
        self.last_root_iterations = iterations;
        Ok(ForceWork {
            work_units: self.sampled.real_len() + iterations,
            scalar_transforms: 3,
        })
    }
}

fn validate(domain: Domain, sampled: Layout) -> Result<(), SolverError> {
    if domain.lengths() != [1.0; 3] || domain.viscosity() != 1.0 {
        return Err(SolverError::InvalidDomain);
    }
    if sampled
        .dimensions()
        .iter()
        .zip(domain.layout().dimensions())
        .any(|(&a, b)| a < b)
    {
        return Err(SolverError::InvalidDomain);
    }
    Ok(())
}

fn buffer<T: Clone>(length: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    storage.resize(length, value);
    Ok(storage)
}
