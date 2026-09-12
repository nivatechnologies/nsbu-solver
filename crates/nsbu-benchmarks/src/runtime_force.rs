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

/// Integration-only prescribed-force policy. Diagnostics always build their explicit providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMode {
    /// Evaluate the original force on every RHS invocation.
    Direct,
    /// Reuse original-force coefficients at the five clocks of one admitted attempt.
    AttemptCached,
}

/// Opaque single-owner storage for the integration-only cached provider.
pub struct AttemptCacheOwner(Box<[AttemptForceCache]>);

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
    /// Attempt-local cache in one fallibly allocated slot; construction never nests this variant.
    AttemptCached(AttemptCacheOwner),
}

impl std::fmt::Debug for RunForce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple(match self {
                Self::Serial(_) => "Serial",
                Self::Parallel(_) => "Parallel",
                Self::AttemptCached(_) => "AttemptCached",
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

    pub(crate) fn integration_limits(
        self,
        domain: Domain,
        mode: IntegrationMode,
    ) -> Result<ForceLimits, SolverError> {
        match mode {
            IntegrationMode::Direct => self.limits(domain),
            IntegrationMode::AttemptCached => boxed_cache_limits(self, domain),
        }
    }

    pub(crate) fn build_integration(
        self,
        domain: Domain,
        mode: IntegrationMode,
        cap: usize,
    ) -> Result<RunForce, SolverError> {
        match mode {
            IntegrationMode::Direct => self.build(domain, cap),
            IntegrationMode::AttemptCached => {
                let limits = boxed_cache_limits(self, domain)?;
                if limits.storage_bytes > cap {
                    return Err(SolverError::ResourceLimit);
                }
                let cache_limits = self.attempt_cache_limits(domain)?;
                let cache = self.build_attempt_cache(domain, cache_limits.storage_bytes)?;
                let mut owner = Vec::new();
                owner
                    .try_reserve_exact(1)
                    .map_err(|_| SolverError::AllocationFailed)?;
                owner.push(cache);
                Ok(RunForce::AttemptCached(AttemptCacheOwner(
                    owner.into_boxed_slice(),
                )))
            }
        }
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

fn boxed_cache_limits(settings: ForceSettings, domain: Domain) -> Result<ForceLimits, SolverError> {
    boxed(settings.attempt_cache_limits(domain)?)
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
            Self::AttemptCached(cache) => cache
                .0
                .first()
                .and_then(PrescribedForce::limits)
                .and_then(|limits| boxed(limits).ok()),
        }
    }

    fn begin_attempt(
        &mut self,
        clock: TickClock,
        ticks: u128,
        limit: ForceLimits,
    ) -> Result<(), SolverError> {
        match self {
            Self::Serial(_) | Self::Parallel(_) => Ok(()),
            Self::AttemptCached(cache) => {
                let force = cache.0.first_mut().ok_or(SolverError::InvalidPayload)?;
                let inner = force.limits().ok_or(SolverError::UnknownProviderCost)?;
                if limit != boxed(inner)? {
                    force.invalidate();
                    return Err(SolverError::ProviderBudgetExceeded);
                }
                force.begin_attempt(clock, ticks, inner)
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
            Self::AttemptCached(cache) => {
                let force = cache.0.first_mut().ok_or(SolverError::InvalidPayload)?;
                let inner = force.limits().ok_or(SolverError::UnknownProviderCost)?;
                if limits != boxed(inner)? {
                    return Err(SolverError::ProviderBudgetExceeded);
                }
                force.evaluate(clock, inner, output)
            }
        }
    }
}

fn boxed(limits: ForceLimits) -> Result<ForceLimits, SolverError> {
    Ok(ForceLimits {
        storage_bytes: limits
            .storage_bytes
            .checked_add(64)
            .ok_or(SolverError::SizeOverflow)?,
        ..limits
    })
}

impl RunForce {
    pub(crate) fn cache_work(&self) -> Option<AttemptCacheWork> {
        match self {
            Self::AttemptCached(cache) => cache.0.first().map(AttemptForceCache::work),
            Self::Serial(_) | Self::Parallel(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_boxed_begin_invalidates_the_inner_attempt() {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let settings = ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        };
        let limits = settings
            .integration_limits(domain, IntegrationMode::AttemptCached)
            .unwrap();
        let mut force = settings
            .build_integration(domain, IntegrationMode::AttemptCached, limits.storage_bytes)
            .unwrap();
        let clock = TickClock::from_rest(-20, 8192).unwrap();
        force.begin_attempt(clock, 128, limits).unwrap();
        let stage = clock.stages(128).unwrap()[0];
        let mut output =
            std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
        force
            .evaluate(stage, limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        let mut wrong = limits;
        wrong.work_units -= 1;
        assert_eq!(
            force.begin_attempt(clock, 128, wrong),
            Err(SolverError::ProviderBudgetExceeded)
        );
        assert!(matches!(
            force.evaluate(stage, limits, output.each_mut().map(Vec::as_mut_slice)),
            Err(SolverError::ProviderBudgetExceeded)
        ));

        for workers in [0, 1] {
            let settings = ForceSettings {
                samples: Layout::new([4; 3]).unwrap(),
                workers,
            };
            let limits = settings.limits(domain).unwrap();
            let mut force = settings.build(domain, limits.storage_bytes).unwrap();
            assert!(!format!("{force:?}").is_empty());
            let mut wrong = limits;
            wrong.work_units -= 1;
            assert!(matches!(
                force.evaluate(stage, wrong, output.each_mut().map(Vec::as_mut_slice)),
                Err(SolverError::ProviderBudgetExceeded)
            ));
        }
    }
}
