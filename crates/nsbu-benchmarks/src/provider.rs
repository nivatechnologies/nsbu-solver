//! Bounded sampled v2 forcing. Sampling is a numerical approximation, never a qualification claim.
pub mod parallel;
pub mod reduced;
use crate::{fields, time::BenchmarkTime};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{transfer, FftBackend, FftCatalog, FftPlan, FftWorkspace},
    Complex64, SolverError,
};

/// Independent force samples and normalized FFT coefficients on a caller-selected evaluation grid.
#[derive(Debug)]
pub struct V2Force {
    retained: Layout,
    sampled: Layout,
    plan: FftPlan,
    workspace: FftWorkspace,
    physical: [Vec<f64>; 3],
    spectral: Vec<Complex64>,
    limits: ForceLimits,
    last_root_iterations: usize,
    axial: Vec<Option<fields::axial::AxialRoot>>,
}

impl V2Force {
    /// Full owned storage and finite work declaration, before allocating any provider buffer.
    /// A work unit is one fixed degree-four field assembly or one safeguarded root iteration.
    /// Each point reserves one complete assembly plus 128 scalar iterations. Reusing the
    /// identical axial jet lowers actual root work; the conservative maximum stays unchanged.
    /// Each call rebuilds its checked plane cache and performs three FFTs.
    pub fn preflight(domain: Domain, sampled: Layout) -> Result<ForceLimits, SolverError> {
        Self::preflight_inner(domain, sampled, None)
    }

    /// Full reservation with immutable FFT plans owned by an enclosing execution catalog.
    pub fn preflight_with_catalog(
        domain: Domain,
        sampled: Layout,
        catalog: &FftCatalog,
    ) -> Result<ForceLimits, SolverError> {
        Self::preflight_inner(domain, sampled, Some(catalog))
    }

    /// Workspace-only reservation for an enclosing execution-owned backend catalog.
    pub fn preflight_with_fft_backend(
        domain: Domain,
        sampled: Layout,
        backend: FftBackend,
    ) -> Result<ForceLimits, SolverError> {
        let fft = FftPlan::reservation_with_shared_backend(sampled, backend)?;
        Self::preflight_parts(domain, sampled, fft)
    }

    fn preflight_inner(
        domain: Domain,
        sampled: Layout,
        catalog: Option<&FftCatalog>,
    ) -> Result<ForceLimits, SolverError> {
        let fft = match catalog {
            Some(catalog) => FftPlan::reservation_from_catalog(sampled, catalog)?,
            None => FftPlan::reservation(sampled)?,
        };
        Self::preflight_parts(domain, sampled, fft)
    }

    fn preflight_parts(
        domain: Domain,
        sampled: Layout,
        fft: usize,
    ) -> Result<ForceLimits, SolverError> {
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
        let axial_bytes = sampled.dimensions()[2]
            .checked_mul(std::mem::size_of::<Option<fields::axial::AxialRoot>>())
            .ok_or(SolverError::SizeOverflow)?;
        // Includes allocator header/rounding allowance for twelve owned allocations.
        let storage_bytes = fft
            .checked_add(buffers)
            .and_then(|n| n.checked_add(axial_bytes))
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

    /// Admit the complete reservation before any allocation; sampling grids remain explicit.
    pub fn new(domain: Domain, sampled: Layout, cap: usize) -> Result<Self, SolverError> {
        let limits = Self::preflight(domain, sampled)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let (plan, workspace) = FftPlan::new(sampled, cap)?;
        Self::allocate(domain, sampled, limits, plan, workspace)
    }

    /// Allocate mutable provider storage while sharing an admitted immutable FFT catalog.
    pub fn new_with_catalog(
        domain: Domain,
        sampled: Layout,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::preflight_with_catalog(domain, sampled, catalog)?;
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
            limits,
            last_root_iterations: 0,
            axial: buffer(sampled.dimensions()[2], None)?,
        })
    }

    /// Root iterations actually used once per active axial plane in the latest successful pass.
    /// Repeated spatial uses of an already computed jet are not counted as new solves.
    pub fn last_root_iterations(&self) -> usize {
        self.last_root_iterations
    }

    fn sample(&mut self, time: BenchmarkTime) -> Result<usize, SolverError> {
        let [nx, ny, nz] = self.sampled.dimensions();
        let mut iterations = 0;
        for (k, slot) in self.axial.iter_mut().enumerate() {
            *slot = fields::axial::AxialRoot::new(k as f64 / nz as f64, time)
                .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
            iterations += slot.map_or(0, fields::axial::AxialRoot::iterations);
        }
        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let point = [
                        i as f64 / nx as f64,
                        j as f64 / ny as f64,
                        k as f64 / nz as f64,
                    ];
                    let index = (i * ny + j) * nz + k;
                    self.sample_point(point, index, k, time)?;
                }
            }
        }
        self.last_root_iterations = iterations;
        Ok(self.sampled.real_len() + iterations)
    }
    fn sample_point(
        &mut self,
        point: [f64; 3],
        index: usize,
        axial_index: usize,
        time: BenchmarkTime,
    ) -> Result<(), SolverError> {
        let sample = fields::axial::evaluate(point, time, self.axial[axial_index].as_ref())
            .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        for (component, value) in self.physical.iter_mut().zip(sample.force) {
            component[index] = value;
        }
        Ok(())
    }
}

impl V2Force {
    // Both sampling backends write the original global array order before the same FFT path.
    fn transform(&mut self, output: [&mut [Complex64]; 3]) -> Result<(), SolverError> {
        for (physical, coefficients) in self.physical.iter().zip(output) {
            self.plan
                .forward(physical, &mut self.spectral, &mut self.workspace)?;
            transfer(self.sampled, self.retained, &self.spectral, coefficients)?;
        }
        Ok(())
    }
}

impl PrescribedForce for V2Force {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }

    fn evaluate(
        &mut self,
        time: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limit != self.limits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        if output
            .iter()
            .any(|component| component.len() != self.retained.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let time = BenchmarkTime::new(time).map_err(|_| SolverError::InvalidClock)?;
        let work_units = self.sample(time)?;
        self.transform(output)?;
        Ok(ForceWork {
            work_units,
            scalar_transforms: 3,
        })
    }
}

fn buffer<T: Clone>(length: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    storage.resize(length, value);
    Ok(storage)
}
