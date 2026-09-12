//! Harness-local Run-equivalent doubled-grid conservative balance observer.
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    diagnostics::{balances, conservative::ConservativeWorkspace},
    domain::{Domain, Layout, SpectralState},
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::{transfer, FftBackend, FftCatalog},
    Complex64, SolverError,
};
use std::time::Instant;

pub struct ObserverResult {
    pub balance: balances::BalanceSample,
    pub force_seconds: f64,
    pub conservative_seconds: f64,
    pub transfer_measure_seconds: f64,
}

pub struct ReducedObserver {
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

impl ReducedObserver {
    pub fn preflight(
        source: Domain,
        samples: Layout,
        workers: usize,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force = ParallelReducedV2Force::preflight_with_fft_backend(
            diagnostic, samples, workers, backend,
        )?;
        let fields = diagnostic
            .layout()
            .half_len()
            .checked_mul(10 * size_of::<Complex64>())
            .ok_or(SolverError::SizeOverflow)?;
        let force_external = force
            .storage_bytes
            .checked_sub(size_of::<ParallelReducedV2Force>())
            .ok_or(SolverError::SizeOverflow)?;
        ConservativeWorkspace::reservation_with_fft_backend(source, backend)?
            .checked_add(force_external)
            .and_then(|n| n.checked_add(fields))
            .and_then(|n| n.checked_add(size_of::<Self>() + 10 * 64))
            .ok_or(SolverError::SizeOverflow)
    }

    pub fn new(
        source: Domain,
        samples: Layout,
        workers: usize,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<Self, SolverError> {
        if Self::preflight(source, samples, workers, catalog.backend())? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force_limits = ParallelReducedV2Force::preflight_with_fft_backend(
            diagnostic,
            samples,
            workers,
            catalog.backend(),
        )?;
        let force = ParallelReducedV2Force::new_with_catalog(
            diagnostic,
            samples,
            workers,
            catalog,
            force_limits.storage_bytes,
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

    pub fn sample(&mut self, state: &SpectralState) -> Result<ObserverResult, SolverError> {
        if state.plan().domain() != self.source {
            return Err(SolverError::InvalidPayload);
        }
        let force_started = Instant::now();
        self.force
            .begin_attempt(state.clock(), 2, self.force_limits)?;
        self.force.evaluate(
            state.clock(),
            self.force_limits,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        let force_seconds = force_started.elapsed().as_secs_f64();
        let conservative_started = Instant::now();
        let [x, y, z] = &mut self.conservative;
        self.products.evaluate(
            [
                state.component(0)?,
                state.component(1)?,
                state.component(2)?,
            ],
            self.forcing.each_ref().map(Vec::as_slice),
            [x, y, z],
            &mut self.pressure,
        )?;
        let conservative_seconds = conservative_started.elapsed().as_secs_f64();
        let transfer_started = Instant::now();
        for (input, output) in (0..3).zip(&mut self.padded) {
            transfer(
                self.source.layout(),
                self.diagnostic.layout(),
                state.component(input)?,
                output,
            )?;
        }
        let balance = balances::measure(
            self.diagnostic,
            self.padded.each_ref().map(Vec::as_slice),
            self.forcing.each_ref().map(Vec::as_slice),
            self.conservative.each_ref().map(Vec::as_slice),
        )?;
        Ok(ObserverResult {
            balance,
            force_seconds,
            conservative_seconds,
            transfer_measure_seconds: transfer_started.elapsed().as_secs_f64(),
        })
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
