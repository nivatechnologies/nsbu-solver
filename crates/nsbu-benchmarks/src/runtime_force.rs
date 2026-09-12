//! Bounded v2 force selection for runtime trajectories.
//!
//! This adapter keeps the serial and persistent-worker providers behind one small
//! configuration surface. It carries no qualification or provenance claim.
use crate::provider::{parallel::ParallelV2Force, V2Force};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64, SolverError,
};

mod attempt_cache;
pub use attempt_cache::{AttemptCacheWork, AttemptForceCache};

/// Sample-grid and worker selection for the exact-v2 prescribed force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceSettings {
    /// Spatial grid used to sample the prescribed force.
    pub samples: Layout,
    /// Zero selects the serial provider; positive values select persistent workers.
    pub workers: usize,
}

/// The selected bounded v2 force implementation.
pub enum RunForce {
    /// Serial exact-v2 sampling and FFT.
    Serial(V2Force),
    /// Persistent-worker exact-v2 sampling followed by the serial FFT.
    Parallel(ParallelV2Force),
}

impl std::fmt::Debug for RunForce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple(match self {
                Self::Serial(_) => "Serial",
                Self::Parallel(_) => "Parallel",
            })
            .finish()
    }
}

impl ForceSettings {
    /// Declare provider storage and finite evaluation work for this setting.
    pub fn limits(self, domain: Domain) -> Result<ForceLimits, SolverError> {
        let (limits, concrete_size) = if self.workers == 0 {
            (
                V2Force::preflight(domain, self.samples)?,
                std::mem::size_of::<V2Force>(),
            )
        } else {
            (
                ParallelV2Force::preflight(domain, self.samples, self.workers)?,
                std::mem::size_of::<ParallelV2Force>(),
            )
        };
        adjust_limits(limits, concrete_size)
    }

    /// Construct the selected provider after checking its complete wrapper reservation.
    pub fn build(self, domain: Domain, cap: usize) -> Result<RunForce, SolverError> {
        let limits = self.limits(domain)?;
        if limits.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        if self.workers == 0 {
            Ok(RunForce::Serial(V2Force::new(domain, self.samples, cap)?))
        } else {
            Ok(RunForce::Parallel(ParallelV2Force::new(
                domain,
                self.samples,
                self.workers,
                cap,
            )?))
        }
    }

    /// Declare the opt-in, attempt-local five-clock cache around this exact-v2 provider.
    pub fn attempt_cache_limits(self, domain: Domain) -> Result<ForceLimits, SolverError> {
        AttemptForceCache::preflight(domain, self)
    }

    /// Build the opt-in attempt-local cache after checking its complete reservation.
    pub fn build_attempt_cache(
        self,
        domain: Domain,
        cap: usize,
    ) -> Result<AttemptForceCache, SolverError> {
        AttemptForceCache::new(domain, self, cap)
    }

    /// Return settings for a checked two-times sample grid with the same worker count.
    pub fn double_grid(self) -> Result<Self, SolverError> {
        let [x, y, z] = self.samples.dimensions();
        let dimensions = [
            x.checked_mul(2).ok_or(SolverError::SizeOverflow)?,
            y.checked_mul(2).ok_or(SolverError::SizeOverflow)?,
            z.checked_mul(2).ok_or(SolverError::SizeOverflow)?,
        ];
        Ok(Self {
            samples: Layout::new(dimensions)?,
            workers: self.workers,
        })
    }
}

fn adjust_limits(limits: ForceLimits, concrete_size: usize) -> Result<ForceLimits, SolverError> {
    let delta = std::mem::size_of::<RunForce>()
        .checked_sub(concrete_size)
        .ok_or(SolverError::SizeOverflow)?;
    Ok(ForceLimits {
        storage_bytes: limits
            .storage_bytes
            .checked_add(delta)
            .ok_or(SolverError::SizeOverflow)?,
        ..limits
    })
}

impl PrescribedForce for RunForce {
    fn limits(&self) -> Option<ForceLimits> {
        match self {
            Self::Serial(force) => {
                adjust_limits(force.limits()?, std::mem::size_of::<V2Force>()).ok()
            }
            Self::Parallel(force) => {
                adjust_limits(force.limits()?, std::mem::size_of::<ParallelV2Force>()).ok()
            }
        }
    }

    fn evaluate(
        &mut self,
        clock: TickClock,
        limits: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        match self {
            Self::Serial(force) => {
                let concrete = force.limits().ok_or(SolverError::UnknownProviderCost)?;
                let declared = adjust_limits(concrete, std::mem::size_of::<V2Force>())?;
                if limits != declared {
                    return Err(SolverError::ProviderBudgetExceeded);
                }
                force.evaluate(clock, concrete, output)
            }
            Self::Parallel(force) => {
                let concrete = force.limits().ok_or(SolverError::UnknownProviderCost)?;
                let declared = adjust_limits(concrete, std::mem::size_of::<ParallelV2Force>())?;
                if limits != declared {
                    return Err(SolverError::ProviderBudgetExceeded);
                }
                force.evaluate(clock, concrete, output)
            }
        }
    }
}
