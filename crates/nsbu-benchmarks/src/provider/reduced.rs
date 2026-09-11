//! Optional serial sampled provider using the reduced degree-three force evaluator.
use crate::{
    reduced_force::axial::{self, AxialRoot},
    time::BenchmarkTime,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    spectral::{transfer, FftPlan, FftWorkspace},
    Complex64, SolverError,
};

#[derive(Debug)]
/// Serial sampled provider using reduced degree-three force arithmetic.
pub struct ReducedV2Force {
    retained: Layout,
    sampled: Layout,
    plan: FftPlan,
    workspace: FftWorkspace,
    physical: [Vec<f64>; 3],
    spectral: Vec<Complex64>,
    roots: Vec<Option<AxialRoot>>,
    limits: ForceLimits,
    last_root_iterations: usize,
}

impl ReducedV2Force {
    /// Declare complete provider storage and bounded per-request work.
    pub fn preflight(domain: Domain, sampled: Layout) -> Result<ForceLimits, SolverError> {
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
        let storage_bytes = FftPlan::reservation(sampled)?
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

    fn transform(&mut self, output: [&mut [Complex64]; 3]) -> Result<(), SolverError> {
        for (physical, coefficients) in self.physical.iter().zip(output) {
            self.plan
                .forward(physical, &mut self.spectral, &mut self.workspace)?;
            transfer(self.sampled, self.retained, &self.spectral, coefficients)?;
        }
        Ok(())
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
