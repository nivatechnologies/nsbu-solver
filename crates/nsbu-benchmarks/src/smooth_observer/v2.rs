//! Runtime v2 balance observer using the independently configured force provider.

use super::{BalanceObserverLimits, BalanceObserverWork, ForceBalance};
use crate::runtime_force::{ForceSettings, RunForce};
use nsbu_solver::{
    domain::Domain,
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

/// Diagnostic observer backed by the runtime v2 force.
pub type V2Observer = ForceBalance<RunForce>;

impl V2Observer {
    /// Declares v2 observer storage and provider work.
    pub fn limits(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
    ) -> Result<BalanceObserverLimits, SolverError> {
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let ds = settings.double_grid()?;
        let fl = ds.limits(diagnostic)?;
        Self::limits_for(source, samples, fl)
    }
    pub(crate) fn limits_with_fft_backend(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        backend: FftBackend,
    ) -> Result<BalanceObserverLimits, SolverError> {
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let doubled = settings.double_grid()?;
        let force_limits = doubled.limits_with_fft_backend(diagnostic, backend)?;
        Self::limits_for_fft_backend(source, samples, force_limits, backend)
    }
    /// Builds a v2 observer with an independently capped force provider.
    pub fn new(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let limits = Self::limits(source, settings, samples)?;
        if cap < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        let force = settings.double_grid()?.build(diagnostic, cap)?;
        Self::new_owned(source, samples, force)
    }
    /// Restores a v2 observer and preserves its charged work ledger.
    pub fn restore(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        work: BalanceObserverWork,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::limits(source, settings, samples)?;
        if cap < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        Self::validate_restored(source, settings, samples, work)?;
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let force = settings.double_grid()?.build(diagnostic, cap)?;
        super::kernel::ForceBalance::allocate(source, limits, work, force)
    }
    pub(crate) fn restore_with_catalog(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        work: BalanceObserverWork,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let limits = Self::limits_with_fft_backend(source, settings, samples, catalog.backend())?;
        if cap < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        Self::validate_restored_with_fft_backend(
            source,
            settings,
            samples,
            work,
            catalog.backend(),
        )?;
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let force = settings
            .double_grid()?
            .build_with_catalog(diagnostic, catalog, cap)?;
        super::kernel::ForceBalance::allocate_with_catalog(source, limits, work, force, catalog)
    }
    /// Checks restored v2 counters against declared upper bounds.
    pub fn validate_restored(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        work: BalanceObserverWork,
    ) -> Result<(), SolverError> {
        Self::validate_restored_inner(source, settings, samples, work, None)
    }

    pub(crate) fn validate_restored_with_fft_backend(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        work: BalanceObserverWork,
        backend: FftBackend,
    ) -> Result<(), SolverError> {
        Self::validate_restored_inner(source, settings, samples, work, Some(backend))
    }

    fn validate_restored_inner(
        source: Domain,
        settings: ForceSettings,
        samples: usize,
        work: BalanceObserverWork,
        backend: Option<FftBackend>,
    ) -> Result<(), SolverError> {
        let limits = match backend {
            Some(backend) => Self::limits_with_fft_backend(source, settings, samples, backend)?,
            None => Self::limits(source, settings, samples)?,
        };
        if work.samples > limits.samples {
            return Err(SolverError::InvalidPayload);
        }
        let diagnostic =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                source,
            )?;
        let doubled = settings.double_grid()?;
        let fl = match backend {
            Some(backend) => doubled.limits_with_fft_backend(diagnostic, backend)?,
            None => doubled.limits(diagnostic)?,
        };
        let min_work = settings
            .double_grid()?
            .samples
            .real_len()
            .checked_mul(work.samples)
            .ok_or(SolverError::SizeOverflow)?;
        let max_work = fl
            .work_units
            .checked_mul(work.samples)
            .ok_or(SolverError::SizeOverflow)?;
        let transforms = fl
            .scalar_transforms
            .checked_add(9)
            .and_then(|n| n.checked_mul(work.samples))
            .ok_or(SolverError::SizeOverflow)?;
        if work.work_units < min_work
            || work.work_units > max_work
            || work.scalar_transforms != transforms
        {
            Err(SolverError::InvalidPayload)
        } else {
            Ok(())
        }
    }
}
