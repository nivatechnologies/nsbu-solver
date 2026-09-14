//! Experimental read-only conservative balance observer for decoded snapshot coefficients.
//!
//! Source provenance: the call sequence and reservation scheme are adapted from the
//! reviewed Run-equivalent observer at
//! `evidence/p10/avx-w3-n256-integration-20260912/harness/src/observer.rs`. This copy
//! borrows already-decoded coefficients and an exact external `TickClock` instead of a
//! live `SpectralState`. It never integrates, resets, or mutates observed state.
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    diagnostics::{balances, conservative::ConservativeWorkspace},
    domain::{validate_spectrum, Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::{transfer, FftBackend, FftCatalog},
    Complex64, SolverError,
};

pub const MANIFEST_HEADER_ALLOWANCE: usize = 1024 * 1024;
pub const SNAPSHOT_PREFIX_ALLOWANCE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverLedger {
    pub backend: FftBackend,
    pub catalog_bytes: usize,
    pub force_storage_bytes: usize,
    pub conservative_bytes: usize,
    pub observer_array_bytes: usize,
    pub snapshot_state_bytes: usize,
    pub header_allowance_bytes: usize,
    pub total_bytes: usize,
}

pub struct OfflineObserver {
    source: Domain,
    diagnostic: Domain,
    force_limits: ForceLimits,
    force: ParallelReducedV2Force,
    products: ConservativeWorkspace,
    forcing: [Vec<Complex64>; 3],
    conservative: [Vec<Complex64>; 3],
    padded: [Vec<Complex64>; 3],
    pressure: Vec<Complex64>,
}

impl OfflineObserver {
    /// Exact complete-reservation ledger before any observer allocation.
    /// Accounts the shared FFT catalog, the resident decoded snapshot, force sampling,
    /// the conservative workspace, observer scratch arrays and header allowances.
    pub fn preflight(
        source: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
        identity_len: usize,
    ) -> Result<ObserverLedger, SolverError> {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let catalog_bytes = FftCatalog::reservation(backend)?;
        let force_limits = ParallelReducedV2Force::preflight_with_fft_backend(
            diagnostic, samples, workers, backend,
        )?;
        let force_external = force_limits
            .storage_bytes
            .checked_sub(size_of::<ParallelReducedV2Force>())
            .ok_or(SolverError::SizeOverflow)?;
        let conservative_bytes =
            ConservativeWorkspace::reservation_with_fft_backend(source, backend)?;
        let observer_array_bytes = diagnostic
            .layout()
            .half_len()
            .checked_mul(10 * size_of::<Complex64>())
            .ok_or(SolverError::SizeOverflow)?;
        let snapshot_state_bytes = source
            .layout()
            .half_len()
            .checked_mul(3 * size_of::<Complex64>())
            .and_then(|bytes| bytes.checked_add(identity_len))
            .and_then(|bytes| bytes.checked_add(SNAPSHOT_PREFIX_ALLOWANCE))
            .ok_or(SolverError::SizeOverflow)?;
        let header_allowance_bytes = MANIFEST_HEADER_ALLOWANCE
            .checked_add(size_of::<Self>() + 10 * 64)
            .ok_or(SolverError::SizeOverflow)?;
        let total_bytes = [
            catalog_bytes,
            force_external,
            conservative_bytes,
            observer_array_bytes,
            snapshot_state_bytes,
            header_allowance_bytes,
        ]
        .into_iter()
        .try_fold(0_usize, |total, bytes| {
            total.checked_add(bytes).ok_or(SolverError::SizeOverflow)
        })?;
        Ok(ObserverLedger {
            backend,
            catalog_bytes,
            force_storage_bytes: force_limits.storage_bytes,
            conservative_bytes,
            observer_array_bytes,
            snapshot_state_bytes,
            header_allowance_bytes,
            total_bytes,
        })
    }

    /// Allocate only after the supplied ledger is re-derived, field by field, from the
    /// actual construction inputs (source, samples, workers, catalog backend, snapshot
    /// identity length) and then fits the caller cap. A forged, stale or cross-profile
    /// ledger is rejected with `InvalidPayload` before any observer allocation, so a
    /// shrunken `total_bytes` cannot bypass the cap.
    pub fn new(
        source: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        identity_len: usize,
        ledger: &ObserverLedger,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let expected = Self::preflight(source, samples, workers, catalog.backend(), identity_len)?;
        if *ledger != expected {
            return Err(SolverError::InvalidPayload);
        }
        if ledger.total_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force_limits = ParallelReducedV2Force::preflight_with_fft_backend(
            diagnostic,
            samples,
            workers,
            catalog.backend(),
        )?;
        if force_limits.storage_bytes != ledger.force_storage_bytes {
            return Err(SolverError::InvalidPayload);
        }
        let force = ParallelReducedV2Force::new_with_catalog(
            diagnostic,
            samples,
            workers,
            catalog,
            ledger.force_storage_bytes,
        )?;
        let products = ConservativeWorkspace::new_with_catalog(
            source,
            catalog,
            ConservativeWorkspace::reservation_with_catalog(source, catalog)?,
        )?;
        let length = diagnostic.layout().half_len();
        Ok(Self {
            source,
            diagnostic,
            force_limits,
            force,
            products,
            forcing: field(length)?,
            conservative: field(length)?,
            padded: field(length)?,
            pressure: values(length)?,
        })
    }

    /// Measure conservative balances at the exact clock from borrowed, validated coefficients.
    /// The inputs are only read; the same library kernels and call order as the existing
    /// `ReducedObserver::sample` produce the same balance quantities on the doubled grid.
    pub fn observe(
        &mut self,
        clock: TickClock,
        velocity: [&[Complex64]; 3],
    ) -> Result<balances::BalanceSample, SolverError> {
        if velocity
            .iter()
            .any(|values| values.len() != self.source.layout().half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        for values in velocity {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        self.force.begin_attempt(clock, 2, self.force_limits)?;
        self.force.evaluate(
            clock,
            self.force_limits,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        if self.force.is_terminated() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        let [x, y, z] = &mut self.conservative;
        self.products.evaluate(
            velocity,
            self.forcing.each_ref().map(Vec::as_slice),
            [x, y, z],
            &mut self.pressure,
        )?;
        for (axis, output) in (0..3).zip(&mut self.padded) {
            transfer(
                self.source.layout(),
                self.diagnostic.layout(),
                velocity[axis],
                output,
            )?;
        }
        balances::measure(
            self.diagnostic,
            self.padded.each_ref().map(Vec::as_slice),
            self.forcing.each_ref().map(Vec::as_slice),
            self.conservative.each_ref().map(Vec::as_slice),
        )
    }
}

fn field(length: usize) -> Result<[Vec<Complex64>; 3], SolverError> {
    Ok([values(length)?, values(length)?, values(length)?])
}

fn values(length: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(length, Complex64::new(0.0, 0.0));
    Ok(values)
}
