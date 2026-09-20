//! Read-only offline analytical reference observer for decoded captured states.
//!
//! The observer borrows already-decoded coefficients and an exact external
//! `TickClock`; it never integrates, resets, replaces, injects, resumes,
//! recenters, aligns or phase-shifts any state. It reuses the reviewed kernels
//! in the reviewed order: prescribed-force evaluation and the conservative
//! pressure construction on the doubled grid (as in the reviewed offline
//! balance observer), `DerivativeWorkspace` sampling of the retained/actual
//! bands, the reviewed independent reference evaluator for the analytical side,
//! and the reviewed local/regional error collectors for the findings.
use nsbu_benchmarks::{
    fields::reference::ReferenceEvaluation,
    provider::parallel_reduced::ParallelReducedV2Force,
    regions::SpatialRegion,
    time::BenchmarkTime,
};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace,
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError},
        physical::PhysicalQuantity,
    },
    domain::{validate_spectrum, Domain, Layout, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::FftCatalog,
    Complex64, SolverError,
};

use crate::{
    cache,
    ledger::{self, AdmissionInputs, ObserverLedger, WorkLedger},
    quantity,
};

pub(crate) struct ReferenceObserver {
    source: Domain,
    velocity_samples: Layout,
    pressure_samples: Layout,
    force_limits: ForceLimits,
    force: ParallelReducedV2Force,
    products: ConservativeWorkspace,
    velocity_sampler: DerivativeWorkspace,
    pressure_sampler: DerivativeWorkspace,
    forcing: [Vec<Complex64>; 3],
    conservative: [Vec<Complex64>; 3],
    pressure: Vec<Complex64>,
    actual: Vec<f64>,
    scratch: Vec<f64>,
    velocity_errors: Vec<f64>,
    velocity_references: Vec<f64>,
    pressure_errors: Vec<f64>,
    pressure_references: Vec<f64>,
}

pub(crate) struct RegionMeasurement {
    pub region: &'static str,
    pub error: SampledError,
}

pub(crate) struct QuantityMeasurement {
    pub name: &'static str,
    pub quantity: &'static str,
    pub components: usize,
    pub transforms: usize,
    pub layout: Layout,
    pub global: LocalError,
    pub regions: Vec<RegionMeasurement>,
    pub grid_complete: bool,
    pub root_work_charged: usize,
    pub error_peak: quantity::FieldPeak,
    pub relative_peak: quantity::FieldPeak,
    pub reference_peak: quantity::FieldPeak,
    pub actual_peak: quantity::FieldPeak,
    pub peak_height_error: f64,
    pub peak_distance: [usize; 3],
}

pub(crate) struct Observations {
    pub quantities: Vec<QuantityMeasurement>,
    pub pressure_gauge_mean: f64,
    pub actual_pressure_lattice_mean: f64,
    pub provider_root_iterations: usize,
    pub executed_scalar_transforms: usize,
}

impl ReferenceObserver {
    /// Exact complete ledger before any allocation, re-checked at construction.
    pub fn preflight(
        inputs: AdmissionInputs,
    ) -> Result<(ObserverLedger, WorkLedger), SolverError> {
        ledger::preflight(inputs)
    }

    /// Allocate only after the supplied ledger is re-derived, field by field, from
    /// the actual construction inputs and then fits the caller cap. A forged,
    /// stale or cross-profile ledger is refused before any allocation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inputs: AdmissionInputs,
        catalog: &FftCatalog,
        ledger: &ObserverLedger,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let inputs = AdmissionInputs {
            backend: catalog.backend(),
            ..inputs
        };
        let (expected, _) = ledger::preflight(inputs)?;
        if *ledger != expected {
            return Err(SolverError::InvalidPayload);
        }
        if ledger.total_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let source = inputs.source;
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force_limits = ParallelReducedV2Force::preflight_with_fft_backend(
            diagnostic,
            inputs.force_samples,
            inputs.workers,
            catalog.backend(),
        )?;
        let doubled = diagnostic.layout().half_len();
        let largest = inputs
            .velocity_samples
            .real_len()
            .max(inputs.pressure_samples.real_len());
        Ok(Self {
            force_limits,
            source,
            velocity_samples: inputs.velocity_samples,
            pressure_samples: inputs.pressure_samples,
            force: ParallelReducedV2Force::new_with_catalog(
                diagnostic,
                inputs.force_samples,
                inputs.workers,
                catalog,
                ledger.force_storage_bytes,
            )?,
            products: ConservativeWorkspace::new_with_catalog(
                source,
                catalog,
                ConservativeWorkspace::reservation_with_catalog(source, catalog)?,
            )?,
            velocity_sampler: DerivativeWorkspace::new_from_catalog(
                source,
                inputs.velocity_samples,
                catalog,
                cap,
            )?,
            pressure_sampler: DerivativeWorkspace::new_from_catalog(
                diagnostic,
                inputs.pressure_samples,
                catalog,
                cap,
            )?,
            forcing: field(doubled)?,
            conservative: field(doubled)?,
            pressure: values(doubled)?,
            actual: zeros(largest)?,
            scratch: zeros(largest)?,
            velocity_errors: zeros(inputs.velocity_samples.real_len())?,
            velocity_references: zeros(inputs.velocity_samples.real_len())?,
            pressure_errors: zeros(inputs.pressure_samples.real_len())?,
            pressure_references: zeros(inputs.pressure_samples.real_len())?,
        })
    }

    /// Measure analytical-reference agreement for the borrowed captured state at
    /// one exact synchronized clock. All inputs are read-only; on any failure no
    /// partial report is issued and the caller must publish nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn observe(
        &mut self,
        clock: TickClock,
        velocity: [&[Complex64]; 3],
        velocity_floor: f64,
        pressure_floor: f64,
        root_budget: usize,
    ) -> Result<Observations, SolverError> {
        if velocity
            .iter()
            .any(|values| values.len() != self.source.layout().half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        for values in velocity {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidPayload)?;
        self.evaluate_force(clock)?;
        self.construct_pressure(velocity)?;
        let velocity_cache = cache::velocity_cache(self.velocity_samples, time)?;
        let pressure_cache = cache::pressure_cache(self.pressure_samples, time)?;
        let gauge_mean = cache::lattice_mean(&pressure_cache, 0)?;
        let (velocity_transforms, mut quantities) =
            self.observe_velocity_family(&velocity_cache, velocity, clock, velocity_floor, root_budget)?;
        let (pressure_transforms, pressure_measurements) =
            self.observe_pressure_family(&pressure_cache, gauge_mean, clock, pressure_floor, root_budget)?;
        quantities.extend(pressure_measurements);
        let actual_mean = self.actual_pressure_lattice_mean()?;
        Ok(Observations {
            quantities,
            pressure_gauge_mean: gauge_mean,
            actual_pressure_lattice_mean: actual_mean,
            provider_root_iterations: self.force.last_root_iterations(),
            // The gauge witness samples the constructed pressure once on the
            // pressure lattice; that derivative sample is a scalar transform
            // and is counted here.
            executed_scalar_transforms: velocity_transforms + pressure_transforms + 1,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_velocity_family(
        &mut self,
        cache: &[ReferenceEvaluation],
        velocity: [&[Complex64]; 3],
        clock: TickClock,
        floor: f64,
        root_budget: usize,
    ) -> Result<(usize, Vec<QuantityMeasurement>), SolverError> {
        let side = quantity::ReferenceSide::Velocity(cache);
        let mut transforms = 0_usize;
        let mut measurements = Vec::new();
        for (name, kind) in [
            ("velocity", PhysicalQuantity::Vector),
            ("gradient", PhysicalQuantity::Gradient),
            ("hessian", PhysicalQuantity::Hessian),
            ("vorticity", PhysicalQuantity::Vorticity),
        ] {
            let finding = match kind {
                PhysicalQuantity::Gradient => self.velocity_measure::<9>(
                    name, kind, &velocity, &side, clock, floor, root_budget,
                )?,
                PhysicalQuantity::Hessian => self.velocity_measure::<27>(
                    name, kind, &velocity, &side, clock, floor, root_budget,
                )?,
                _ => self.velocity_measure::<3>(
                    name, kind, &velocity, &side, clock, floor, root_budget,
                )?,
            };
            transforms += finding.transforms;
            measurements.push(measurement(finding));
        }
        Ok((transforms, measurements))
    }

    #[allow(clippy::too_many_arguments)]
    fn velocity_measure<const C: usize>(
        &mut self,
        name: &'static str,
        kind: PhysicalQuantity,
        spectra: &[&[Complex64]],
        side: &quantity::ReferenceSide<'_>,
        clock: TickClock,
        floor: f64,
        root_budget: usize,
    ) -> Result<quantity::QuantityFinding, SolverError> {
        quantity::measure::<C>(
            name,
            kind,
            &mut self.velocity_sampler,
            spectra,
            side,
            self.velocity_samples,
            clock,
            floor,
            root_budget,
            &mut self.actual,
            &mut self.scratch,
            &mut self.velocity_errors,
            &mut self.velocity_references,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_pressure_family(
        &mut self,
        rows: &[[f64; 4]],
        mean: f64,
        clock: TickClock,
        floor: f64,
        root_budget: usize,
    ) -> Result<(usize, Vec<QuantityMeasurement>), SolverError> {
        let side = quantity::ReferenceSide::Pressure { rows, mean };
        let spectra = [&self.pressure[..]];
        let scalar = quantity::measure::<1>(
            "pressure",
            PhysicalQuantity::Scalar,
            &mut self.pressure_sampler,
            &spectra,
            &side,
            self.pressure_samples,
            clock,
            floor,
            root_budget,
            &mut self.actual,
            &mut self.scratch,
            &mut self.pressure_errors,
            &mut self.pressure_references,
        )?;
        let gradient = quantity::measure::<3>(
            "pressure_gradient",
            PhysicalQuantity::ScalarGradient,
            &mut self.pressure_sampler,
            &spectra,
            &side,
            self.pressure_samples,
            clock,
            floor,
            root_budget,
            &mut self.actual,
            &mut self.scratch,
            &mut self.pressure_errors,
            &mut self.pressure_references,
        )?;
        Ok((
            scalar.transforms + gradient.transforms,
            vec![measurement(scalar), measurement(gradient)],
        ))
    }

    fn evaluate_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        self.force.begin_attempt(clock, 2, self.force_limits)?;
        let work = self.force.evaluate(
            clock,
            self.force_limits,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        if self.force.is_terminated()
            || work.work_units > self.force_limits.work_units
            || work.scalar_transforms > self.force_limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }

    fn construct_pressure(&mut self, velocity: [&[Complex64]; 3]) -> Result<(), SolverError> {
        let [x, y, z] = &mut self.conservative;
        self.products.evaluate(
            velocity,
            self.forcing.each_ref().map(Vec::as_slice),
            [x, y, z],
            &mut self.pressure,
        )
    }

    /// Sampled mean of the actually constructed pressure on the pressure lattice;
    /// the construction is mean-zero by design, so this is a gauge witness only.
    fn actual_pressure_lattice_mean(&mut self) -> Result<f64, SolverError> {
        let zero = Derivative::new([0; 3])?;
        let sampled = self.pressure_sampler.sample(&self.pressure, zero)?;
        let mut sum = 0_f64;
        for value in sampled.values {
            if !value.is_finite() {
                return Err(SolverError::ArithmeticResolutionLimited);
            }
            sum += value;
        }
        Ok(sum / sampled.values.len() as f64)
    }
}

pub(crate) fn region_name(region: SpatialRegion) -> &'static str {
    match region {
        SpatialRegion::Core => "core",
        SpatialRegion::Annulus => "annulus",
        SpatialRegion::InteriorOutsideNominal => "interior-outside-nominal",
        SpatialRegion::Collar => "collar",
        SpatialRegion::Exterior => "exterior",
    }
}

fn measurement(finding: quantity::QuantityFinding) -> QuantityMeasurement {
    QuantityMeasurement {
        name: finding.name,
        quantity: quantity_name(finding.quantity),
        components: finding.components,
        transforms: finding.transforms,
        layout: finding.layout,
        global: finding.global,
        regions: finding
            .regions
            .into_iter()
            .map(|(region, error)| RegionMeasurement {
                region: region_name(region),
                error,
            })
            .collect(),
        grid_complete: finding.grid_complete,
        root_work_charged: finding.root_work_charged,
        error_peak: finding.error_peak,
        relative_peak: finding.relative_peak,
        reference_peak: finding.reference_peak,
        actual_peak: finding.actual_peak,
        peak_height_error: finding.peak_height_error,
        peak_distance: finding.peak_distance,
    }
}

fn quantity_name(quantity: PhysicalQuantity) -> &'static str {
    match quantity {
        PhysicalQuantity::Scalar => "scalar",
        PhysicalQuantity::ScalarGradient => "scalar-gradient",
        PhysicalQuantity::Vector => "vector",
        PhysicalQuantity::Gradient => "gradient",
        PhysicalQuantity::Hessian => "hessian",
        PhysicalQuantity::Vorticity => "vorticity",
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

fn zeros(length: usize) -> Result<Vec<f64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(length, 0.0);
    Ok(values)
}
