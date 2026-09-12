//! Bounded, independently owned balance measurements for prescribed force providers.
use nsbu_solver::{
    diagnostics::{
        balances::{measure, BalanceSample},
        conservative::ConservativeWorkspace,
    },
    domain::{Domain, ResourcePlan, SpectralState, TickClock},
    experiment::observer::{BalanceObserver as BalanceObserverContract, ObserverBounds},
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::{transfer, FftBackend, FftCatalog},
    Complex64, SolverError,
};

type Field = [Vec<Complex64>; 3];

/// Complete fixed storage and bounded charged work for a balance observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceObserverLimits {
    /// All heap elements and the observer header, excluding allocator overhead and its state.
    pub storage_bytes: usize,
    /// Maximum successful or failed sample invocations admitted by this observer.
    pub samples: usize,
    /// Maximum provider work units charged across all admitted samples.
    pub work_units: usize,
    /// Maximum scalar transforms charged across all admitted samples.
    pub scalar_transforms: usize,
}

/// Work charged so far. A failed admitted sample retains its full declared charge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BalanceObserverWork {
    /// Number of sample slots consumed.
    pub samples: usize,
    /// Provider work units charged after successful provider reports release unused units.
    pub work_units: usize,
    /// Provider transforms plus nine conservative-product transforms per sample.
    pub scalar_transforms: usize,
}

/// Independent double-grid balance diagnostics for accepted spectral states.
///
/// The observer owns its force provider and every mutable buffer. Sampling only borrows a state,
/// so it cannot alter the right-hand-side provider or any integration workspace.
#[derive(Debug)]
pub struct ForceBalance<F: PrescribedForce> {
    pub(super) source: Domain,
    pub(super) diagnostic: Domain,
    pub(super) limits: BalanceObserverLimits,
    pub(super) force_limits: ForceLimits,
    pub(super) force: F,
    pub(super) products: ConservativeWorkspace,
    pub(super) forcing: Field,
    pub(super) conservative: Field,
    pub(super) padded: Field,
    pub(super) pressure: Vec<Complex64>,
    pub(super) work: BalanceObserverWork,
}

impl<F: PrescribedForce> ForceBalance<F> {
    /// Declare all observer storage and total work before any observer allocation.
    pub(super) fn limits_for(
        source: Domain,
        samples: usize,
        force_limits: ForceLimits,
    ) -> Result<BalanceObserverLimits, SolverError> {
        Self::limits_inner(source, samples, force_limits, None)
    }

    pub(super) fn limits_for_fft_backend(
        source: Domain,
        samples: usize,
        force_limits: ForceLimits,
        backend: FftBackend,
    ) -> Result<BalanceObserverLimits, SolverError> {
        Self::limits_inner(source, samples, force_limits, Some(backend))
    }

    fn limits_inner(
        source: Domain,
        samples: usize,
        force_limits: ForceLimits,
        backend: Option<FftBackend>,
    ) -> Result<BalanceObserverLimits, SolverError> {
        if samples == 0 {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;

        let half = diagnostic.layout().half_len();
        let fields = half
            .checked_mul(10)
            .and_then(|count| count.checked_mul(std::mem::size_of::<Complex64>()))
            .ok_or(SolverError::SizeOverflow)?;
        let workspace = match backend {
            Some(backend) => ConservativeWorkspace::reservation_with_fft_backend(source, backend)?,
            None => ConservativeWorkspace::reservation(source)?,
        }
        .checked_sub(std::mem::size_of::<ConservativeWorkspace>())
        .ok_or(SolverError::SizeOverflow)?;
        let force_external = force_limits
            .storage_bytes
            .saturating_sub(std::mem::size_of::<F>());
        let storage_bytes = fields
            .checked_add(workspace)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .and_then(|bytes| bytes.checked_add(force_external))
            .ok_or(SolverError::SizeOverflow)?;
        let per_sample_transforms = force_limits
            .scalar_transforms
            .checked_add(9)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(BalanceObserverLimits {
            storage_bytes,
            samples,
            work_units: force_limits
                .work_units
                .checked_mul(samples)
                .ok_or(SolverError::SizeOverflow)?,
            scalar_transforms: per_sample_transforms
                .checked_mul(samples)
                .ok_or(SolverError::SizeOverflow)?,
        })
    }

    /// Allocate after the state plan has reserved this observer in its diagnostics class.
    pub(crate) fn new_with_force(
        plan: ResourcePlan,
        samples: usize,
        force: F,
    ) -> Result<Self, SolverError> {
        Self::restore_with_force(plan, samples, BalanceObserverWork::default(), force)
    }

    pub(crate) fn new_owned(source: Domain, samples: usize, force: F) -> Result<Self, SolverError> {
        let limits = Self::limits_for(
            source,
            samples,
            force.limits().ok_or(SolverError::UnknownProviderCost)?,
        )?;
        Self::allocate(source, limits, BalanceObserverWork::default(), force)
    }

    /// Reconstruct fresh diagnostic scratch and restore an exactly charged observer ledger.
    ///
    /// Providers with immutable evaluator state may report their full declared force cost on
    /// every admitted call. Consequently, only the counters need restoration; no provider or
    /// numerical workspace state is serialized. All counter and plan checks happen before any
    /// observer buffer is allocated.
    pub(crate) fn restore_with_force(
        plan: ResourcePlan,
        samples: usize,
        work: BalanceObserverWork,
        force: F,
    ) -> Result<Self, SolverError> {
        let source = plan.domain();
        let limits = Self::limits_for(
            source,
            samples,
            force.limits().ok_or(SolverError::UnknownProviderCost)?,
        )?;
        validate_work_public(
            limits,
            force.limits().ok_or(SolverError::UnknownProviderCost)?,
            work,
        )?;
        if plan.classes()[6] < limits.storage_bytes {
            return Err(SolverError::ResourceLimit);
        }
        Self::allocate(source, limits, work, force)
    }

    pub(crate) fn allocate(
        source: Domain,
        limits: BalanceObserverLimits,
        work: BalanceObserverWork,
        force: F,
    ) -> Result<Self, SolverError> {
        Self::allocate_inner(source, limits, work, force, None)
    }

    pub(crate) fn allocate_with_catalog(
        source: Domain,
        limits: BalanceObserverLimits,
        work: BalanceObserverWork,
        force: F,
        catalog: &FftCatalog,
    ) -> Result<Self, SolverError> {
        Self::allocate_inner(source, limits, work, force, Some(catalog))
    }

    fn allocate_inner(
        source: Domain,
        limits: BalanceObserverLimits,
        work: BalanceObserverWork,
        force: F,
        catalog: Option<&FftCatalog>,
    ) -> Result<Self, SolverError> {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force_limits = force.limits().ok_or(SolverError::UnknownProviderCost)?;
        let half = diagnostic.layout().half_len();
        let zero = Complex64::new(0.0, 0.0);
        let products = match catalog {
            Some(catalog) => ConservativeWorkspace::new_with_catalog(
                source,
                catalog,
                ConservativeWorkspace::reservation_with_catalog(source, catalog)?,
            )?,
            None => {
                ConservativeWorkspace::new(source, ConservativeWorkspace::reservation(source)?)?
            }
        };
        Ok(Self {
            source,
            diagnostic,
            limits,
            force_limits,
            force,
            products,
            forcing: field(half, zero)?,
            conservative: field(half, zero)?,
            padded: field(half, zero)?,
            pressure: filled(half, zero)?,
            work,
        })
    }

    /// Immutable declaration that was admitted before construction.
    pub fn limits_declared(&self) -> BalanceObserverLimits {
        self.limits
    }

    /// Work charged to admitted calls so a run ledger can account for diagnostics separately.
    pub fn consumption(&self) -> BalanceObserverWork {
        self.work
    }

    /// Form a complete double-grid balance sample from an accepted read-only state.
    pub fn sample(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError> {
        if state.plan().domain() != self.source {
            return Err(SolverError::InvalidPayload);
        }
        self.sample_fields(
            state.clock(),
            [
                state.component(0)?,
                state.component(1)?,
                state.component(2)?,
            ],
        )
    }
    // Only accepted-state sampling and the privately bound reconstructed-probe bridge call this.
    pub(crate) fn sample_probe(
        &mut self,
        domain: Domain,
        clock: TickClock,
        velocity: [&[Complex64]; 3],
    ) -> Result<BalanceSample, SolverError> {
        if domain != self.source {
            return Err(SolverError::InvalidPayload);
        }
        self.sample_fields(clock, velocity)
    }
    fn sample_fields(
        &mut self,
        clock: TickClock,
        velocity: [&[Complex64]; 3],
    ) -> Result<BalanceSample, SolverError> {
        self.prepare_force(clock)?;
        let [x, y, z] = &mut self.conservative;
        self.products.evaluate(
            velocity,
            self.forcing.each_ref().map(Vec::as_slice),
            [x, y, z],
            &mut self.pressure,
        )?;
        for (input, output) in velocity.into_iter().zip(&mut self.padded) {
            transfer(
                self.source.layout(),
                self.diagnostic.layout(),
                input,
                output,
            )?;
        }
        measure(
            self.diagnostic,
            self.padded.each_ref().map(Vec::as_slice),
            self.forcing.each_ref().map(Vec::as_slice),
            self.conservative.each_ref().map(Vec::as_slice),
        )
    }
    // Admit and charge the independent provider before forming any diagnostic fields.
    // A failed admitted request retains its declared work; physical state stays borrowed.
    fn prepare_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        if self.work.samples == self.limits.samples
            || self.force.limits() != Some(self.force_limits)
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charge_sample()?;
        let report = self.force.evaluate(
            clock,
            self.force_limits,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        if report.work_units > self.force_limits.work_units
            || report.scalar_transforms > self.force_limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.work.work_units -= self.force_limits.work_units - report.work_units;
        self.work.scalar_transforms -=
            self.force_limits.scalar_transforms - report.scalar_transforms;
        Ok(())
    }

    // Reserve the complete admitted charge before invoking provider code.
    fn charge_sample(&mut self) -> Result<(), SolverError> {
        self.work.samples = self
            .work
            .samples
            .checked_add(1)
            .ok_or(SolverError::SizeOverflow)?;
        self.work.work_units = self
            .work
            .work_units
            .checked_add(self.force_limits.work_units)
            .ok_or(SolverError::SizeOverflow)?;
        self.work.scalar_transforms = self
            .work
            .scalar_transforms
            .checked_add(
                self.force_limits
                    .scalar_transforms
                    .checked_add(9)
                    .ok_or(SolverError::SizeOverflow)?,
            )
            .ok_or(SolverError::SizeOverflow)?;
        Ok(())
    }
}

pub(crate) fn validate_work_public(
    limits: BalanceObserverLimits,
    force_limits: ForceLimits,
    work: BalanceObserverWork,
) -> Result<(), SolverError> {
    if work.samples > limits.samples {
        return Err(SolverError::InvalidPayload);
    }
    let expected_work = force_limits
        .work_units
        .checked_mul(work.samples)
        .ok_or(SolverError::SizeOverflow)?;
    let expected_transforms = force_limits
        .scalar_transforms
        .checked_add(9)
        .and_then(|per_sample| per_sample.checked_mul(work.samples))
        .ok_or(SolverError::SizeOverflow)?;
    if work.work_units != expected_work || work.scalar_transforms != expected_transforms {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

impl<F: PrescribedForce> BalanceObserverContract for ForceBalance<F> {
    /// `ObserverBounds::work_units` preserves the provider's work-unit scale. The nine
    /// conservative scalar transforms have no representation in `ObserverBounds`; callers that
    /// maintain a transform ledger use [`ForceBalance::consumption`] and its documented
    /// `scalar_transforms` charge instead.
    fn bounds(&self) -> Option<ObserverBounds> {
        Some(ObserverBounds {
            storage_bytes: self.limits.storage_bytes,
            work_units: self.force_limits.work_units,
        })
    }

    fn measure(&mut self, state: &SpectralState) -> Result<BalanceSample, SolverError> {
        self.sample(state)
    }
}

pub(crate) fn field(length: usize, value: Complex64) -> Result<Field, SolverError> {
    Ok([
        filled(length, value)?,
        filled(length, value)?,
        filled(length, value)?,
    ])
}

fn filled(length: usize, value: Complex64) -> Result<Vec<Complex64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(length, value);
    Ok(values)
}
