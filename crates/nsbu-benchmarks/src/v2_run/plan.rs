//! Allocation-free admission of the full integration, observation and history owner.
use super::{work, Run, Settings};
use crate::{
    runtime_force::{IntegrationMode, RunForce},
    smooth_observer::{v2::V2Observer, BalanceObserverLimits},
    time::BenchmarkTime,
};
use nsbu_solver::{
    domain::{Epoch, ExtraStorage, ResourcePlan, TickClock},
    experiment::{control::Controller, log::RunHistory},
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs},
    spectral::{FftBackend, FftCatalog},
    SolverError,
};

/// Checked complete resource and work declaration for one exact-v2 run.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    settings: Settings,
    resources: ResourcePlan,
    observer: BalanceObserverLimits,
    integration: [usize; 3],
    fft_backend: FftBackend,
}

impl Plan {
    /// Validate settings and total reservation before allocating grid storage or worker threads.
    pub fn from_rest(settings: Settings, cap: usize) -> Result<Self, SolverError> {
        Self::admit(
            settings,
            IntegrationMode::Direct,
            FftBackend::OwnedRadix,
            cap,
        )
    }

    /// Admit the opt-in attempt-local original-force cache profile from exact rest.
    pub fn from_rest_cached(settings: Settings, cap: usize) -> Result<Self, SolverError> {
        Self::admit(
            settings,
            IntegrationMode::AttemptCached,
            FftBackend::OwnedRadix,
            cap,
        )
    }

    /// Admit a from-rest run with an explicit immutable FFT arithmetic backend.
    pub fn from_rest_with_fft(
        settings: Settings,
        backend: FftBackend,
        cap: usize,
    ) -> Result<Self, SolverError> {
        Self::admit(settings, IntegrationMode::Direct, backend, cap)
    }

    /// Admit the attempt-local force cache with an explicit immutable FFT backend.
    pub fn from_rest_cached_with_fft(
        settings: Settings,
        backend: FftBackend,
        cap: usize,
    ) -> Result<Self, SolverError> {
        Self::admit(settings, IntegrationMode::AttemptCached, backend, cap)
    }

    fn admit(
        settings: Settings,
        mode: IntegrationMode,
        fft_backend: FftBackend,
        cap: usize,
    ) -> Result<Self, SolverError> {
        validate_layout()?;
        validate_settings(settings)?;
        let force_limits = settings.force.integration_limits_with_fft_backend(
            settings.domain,
            mode,
            fft_backend,
        )?;
        validate_final_step(settings, force_limits.remaining_divisor)?;
        let calls = settings
            .configuration
            .method
            .rhs_calls()
            .checked_mul(settings.configuration.limits.maximum_attempts)
            .ok_or(SolverError::SizeOverflow)?;
        let integration = [
            calls,
            force_limits
                .work_units
                .checked_mul(calls)
                .ok_or(SolverError::SizeOverflow)?,
            force_limits
                .scalar_transforms
                .checked_add(10)
                .and_then(|n| n.checked_mul(calls))
                .ok_or(SolverError::SizeOverflow)?,
        ];
        let observer = V2Observer::limits_with_fft_backend(
            settings.domain,
            settings.force,
            settings.configuration.limits.maximum_attempts,
            fft_backend,
        )?;
        let force = SpectralRhs::<RunForce>::reservation_with_fft_backend(
            settings.domain,
            force_limits,
            fft_backend,
        )?;
        let diagnostics = AttemptWorkspace::reservation_with_method(
            settings.domain,
            settings.configuration.method,
        )?
        .checked_add(observer.storage_bytes)
        .ok_or(SolverError::SizeOverflow)?;
        let overhead = RunHistory::reservation(settings.configuration)?
            .checked_add(work::reservation(
                settings.configuration.limits.maximum_attempts,
            )?)
            .and_then(|n| n.checked_add(std::mem::size_of::<Run>()))
            .and_then(|n| n.checked_add(4096))
            .ok_or(SolverError::SizeOverflow)?;
        let resources = ResourcePlan::new(
            settings.domain,
            ExtraStorage {
                fft: FftCatalog::reservation(fft_backend)?,
                force,
                diagnostics,
                overhead,
            },
            cap,
            Epoch(0),
        )?;
        Ok(Self {
            settings,
            resources,
            observer,
            integration,
            fft_backend,
        })
    }
    /// Immutable settings bound to this admission.
    pub fn settings(self) -> Settings {
        self.settings
    }
    /// Full numerical and auxiliary storage, including worker planning allowances.
    pub fn resources(self) -> ResourcePlan {
        self.resources
    }
    /// Maximum diagnostic invocations, bounded by the complete attempt allowance.
    pub fn observer_samples(self) -> usize {
        self.observer.samples
    }
    /// Separate doubled-grid diagnostic reservation and total charged-work bound.
    pub fn observer_limits(self) -> BalanceObserverLimits {
        self.observer
    }
    /// Maximum integration RHS calls, provider work units, and scalar transforms.
    pub fn integration_limits(self) -> [usize; 3] {
        self.integration
    }

    /// Immutable transform arithmetic identity for this run and its artifacts.
    pub fn fft_backend(self) -> FftBackend {
        self.fft_backend
    }

    /// Integration-only force policy; diagnostic providers remain explicit and uncached.
    pub fn integration_mode(self) -> IntegrationMode {
        direct_or_cached_with_fft(self.settings, self.resources, self.fft_backend)
    }
}

fn validate_layout() -> Result<(), SolverError> {
    if std::mem::size_of::<RunForce>() != 592
        || std::mem::size_of::<Plan>() != 512
        || std::mem::size_of::<Run>() != 5840
    {
        return Err(SolverError::ResourceLimit);
    }
    Ok(())
}

pub(super) fn direct_or_cached(settings: Settings, resources: ResourcePlan) -> IntegrationMode {
    direct_or_cached_with_fft(settings, resources, FftBackend::OwnedRadix)
}

pub(super) fn direct_or_cached_with_fft(
    settings: Settings,
    resources: ResourcePlan,
    backend: FftBackend,
) -> IntegrationMode {
    let direct = settings
        .force
        .limits_with_fft_backend(settings.domain, backend)
        .and_then(|limits| {
            SpectralRhs::<RunForce>::reservation_with_fft_backend(settings.domain, limits, backend)
        });
    if direct == Ok(resources.classes()[5]) {
        IntegrationMode::Direct
    } else {
        IntegrationMode::AttemptCached
    }
}

pub(super) fn validate_settings(settings: Settings) -> Result<(), SolverError> {
    if settings.initial_clock
        != TickClock::from_rest(
            settings.initial_clock.exponent(),
            settings.initial_clock.target(),
        )?
    {
        return Err(SolverError::InvalidClock);
    }
    BenchmarkTime::new(settings.initial_clock).map_err(|_| SolverError::InvalidClock)?;
    Controller::new(settings.initial_clock, settings.configuration)?;
    if !settings.advective_limit.is_finite() || settings.advective_limit <= 0.0 {
        return Err(SolverError::InvalidStep);
    }
    Ok(())
}

// Remaining time decreases monotonically, so the final attempted interval is the
// strictest known prescribed-force time-scale bound. Advective/error tests still run.
pub(super) fn validate_final_step(settings: Settings, divisor: u128) -> Result<(), SolverError> {
    let limits = settings.configuration.limits;
    let remaining = settings
        .initial_clock
        .target()
        .checked_sub(limits.endpoint)
        .and_then(|n| n.checked_add(limits.step_ticks))
        .ok_or(SolverError::InvalidStep)?;
    if divisor == 0 || limits.step_ticks > remaining / divisor {
        return Err(SolverError::InvalidStep);
    }
    Ok(())
}
