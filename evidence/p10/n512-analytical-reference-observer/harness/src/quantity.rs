//! Sequential actual-side sampling and pointwise analytical comparison per quantity.
//!
//! The component/derivative order and vorticity orientation match the reviewed
//! `diagnostics::physical::quantity::PhysicalQuantity::entry` and the reviewed v2
//! analytical tracking module exactly. Only the captured state is sampled; the
//! reference side is the cached independent evaluator output, never a spectrally
//! reconstructed or reassigned field. The peak witnesses carry typed field and
//! source identity: the actual-field peak, the actual-versus-reference peak
//! height error and the periodic per-axis peak location distance.
use nsbu_benchmarks::{
    fields::reference::ReferenceEvaluation,
    regions::{RegionalTensorErrors, SpatialRegion},
};
use nsbu_solver::{
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
        physical::PhysicalQuantity,
    },
    domain::{Layout, TickClock},
    Complex64, SolverError,
};

/// One quantity's complete findings for one snapshot at one exact clock.
#[derive(Debug)]
pub(crate) struct QuantityFinding {
    pub name: &'static str,
    pub quantity: PhysicalQuantity,
    pub components: usize,
    pub transforms: usize,
    pub layout: Layout,
    pub global: LocalError,
    pub regions: [(SpatialRegion, SampledError); 5],
    pub grid_complete: bool,
    pub root_work_charged: usize,
    pub error_peak: FieldPeak,
    pub relative_peak: FieldPeak,
    pub reference_peak: FieldPeak,
    pub actual_peak: FieldPeak,
    pub peak_height_error: f64,
    pub peak_distance: [usize; 3],
}

/// Reviewed extrema witness contract: first linear maximizer in x-major, z-fast
/// order over finite nonnegative magnitudes; absence is never zero measurement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SampleMaximum {
    pub linear: usize,
    pub index: [usize; 3],
    pub value: f64,
}

/// Peak witness source identity, serialized with every peak location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeakSource {
    Actual,
    Reference,
    Error,
    RelativeError,
}

impl PeakSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Actual => "actual",
            Self::Reference => "reference",
            Self::Error => "error",
            Self::RelativeError => "relative-error",
        }
    }
}

/// Typed field/source identity for one peak witness.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldPeak {
    pub field: &'static str,
    pub source: PeakSource,
    pub maximum: SampleMaximum,
}

/// Borrowed reference side of one comparison: the velocity-family evaluation cache
/// or the four-column pressure rows with the explicitly gauged analytical mean.
pub(crate) enum ReferenceSide<'a> {
    Velocity(&'a [ReferenceEvaluation]),
    Pressure { rows: &'a [[f64; 4]], mean: f64 },
}

impl ReferenceSide<'_> {
    fn component(&self, quantity: PhysicalQuantity, index: usize, point: usize) -> f64 {
        match (self, quantity) {
            (Self::Velocity(cache), PhysicalQuantity::Vector) => cache[point].velocity[index],
            (Self::Velocity(cache), PhysicalQuantity::Gradient) => {
                cache[point].gradient[index / 3][index % 3]
            }
            (Self::Velocity(cache), PhysicalQuantity::Hessian) => {
                cache[point].hessian[index / 9][(index / 3) % 3][index % 3]
            }
            (Self::Velocity(cache), PhysicalQuantity::Vorticity) => cache[point].vorticity[index],
            (Self::Pressure { rows, mean }, PhysicalQuantity::Scalar) => rows[point][0] - mean,
            (Self::Pressure { rows, .. }, PhysicalQuantity::ScalarGradient) => {
                rows[point][1 + index]
            }
            _ => f64::NAN,
        }
    }
}

/// Component index and derivative orders identical to the reviewed entry schedule.
pub(crate) fn entry(quantity: PhysicalQuantity, index: usize) -> (usize, [u8; 3]) {
    let mut orders = [0; 3];
    let component = match quantity {
        PhysicalQuantity::Scalar => 0,
        PhysicalQuantity::ScalarGradient => {
            orders[index] = 1;
            0
        }
        PhysicalQuantity::Vector => index,
        PhysicalQuantity::Gradient => {
            orders[index % 3] = 1;
            index / 3
        }
        PhysicalQuantity::Hessian => {
            orders[(index / 3) % 3] += 1;
            orders[index % 3] += 1;
            index / 9
        }
        PhysicalQuantity::Vorticity => {
            orders[(index + 1) % 3] = 1;
            (index + 2) % 3
        }
    };
    (component, orders)
}

/// Minimum-image distance between two peak locations along one periodic axis.
pub(crate) fn axis_distance(size: usize, actual: usize, reference: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let delta = (actual % size).abs_diff(reference % size);
    delta.min(size - delta)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn measure<const C: usize>(
    name: &'static str,
    quantity: PhysicalQuantity,
    sampler: &mut DerivativeWorkspace,
    spectra: &[&[Complex64]],
    reference: &ReferenceSide<'_>,
    samples: Layout,
    clock: TickClock,
    floor: f64,
    root_budget: usize,
    actual: &mut [f64],
    scratch: &mut [f64],
    errors: &mut [f64],
    references: &mut [f64],
) -> Result<QuantityFinding, SolverError> {
    let points = samples.real_len();
    if actual.len() < points
        || scratch.len() < points
        || errors.len() < points
        || references.len() < points
    {
        return Err(SolverError::InvalidPayload);
    }
    let actual = &mut actual[..points];
    let scratch = &mut scratch[..points];
    let errors = &mut errors[..points];
    let references = &mut references[..points];
    scratch.fill(0.0);
    errors.fill(0.0);
    references.fill(0.0);
    let transforms = sample_components(
        sampler,
        spectra,
        reference,
        quantity,
        actual,
        scratch,
        errors,
        references,
    )?;
    let (global, areas, grid_complete, root_work_charged) =
        collect_statistics::<C>(errors, references, samples, clock, floor, root_budget)?;
    let witnesses = peak_witnesses(errors, references, scratch, samples, floor, name)?;
    Ok(QuantityFinding {
        name,
        quantity,
        components: quantity.components(),
        transforms,
        layout: samples,
        global,
        regions: areas,
        grid_complete,
        root_work_charged,
        error_peak: witnesses.error_peak,
        relative_peak: witnesses.relative_peak,
        reference_peak: witnesses.reference_peak,
        actual_peak: witnesses.actual_peak,
        peak_height_error: witnesses.peak_height_error,
        peak_distance: witnesses.peak_distance,
    })
}

/// Statistics sweep for one component family: the local global error, five
/// regional samples, whether the region budget was exhausted, and how many
/// region samples were retained.
type RegionalSweep = (LocalError, [(SpatialRegion, SampledError); 5], bool, usize);

struct Witnesses {
    error_peak: FieldPeak,
    relative_peak: FieldPeak,
    reference_peak: FieldPeak,
    actual_peak: FieldPeak,
    peak_height_error: f64,
    peak_distance: [usize; 3],
}

#[allow(clippy::too_many_arguments)]
fn sample_components(
    sampler: &mut DerivativeWorkspace,
    spectra: &[&[Complex64]],
    reference: &ReferenceSide<'_>,
    quantity: PhysicalQuantity,
    actual: &mut [f64],
    scratch: &mut [f64],
    errors: &mut [f64],
    references: &mut [f64],
) -> Result<usize, SolverError> {
    let mut transforms = 0_usize;
    for index in 0..quantity.components() {
        let (component, orders) = entry(quantity, index);
        sample_component(
            sampler,
            spectra,
            component,
            orders,
            quantity,
            index,
            actual,
            &mut transforms,
        )?;
        accumulate_points(actual, errors, references, scratch, reference, quantity, index)?;
    }
    Ok(transforms)
}

fn collect_statistics<const C: usize>(
    errors: &[f64],
    references: &[f64],
    samples: Layout,
    clock: TickClock,
    floor: f64,
    root_budget: usize,
) -> Result<RegionalSweep, SolverError> {
    let points = samples.real_len();
    let mut statistics = TensorErrors::<C>::new(points, floor)?;
    let mut regions = RegionalTensorErrors::<C>::new(clock, samples, root_budget, points, floor)
        .map_err(|_| SolverError::InvalidPayload)?;
    for index in 0..points {
        statistics.push_magnitudes(errors[index], references[index])?;
        regions
            .push_magnitudes(errors[index], references[index])
            .map_err(|_| SolverError::InvalidPayload)?;
    }
    let global = match statistics.finish()? {
        SampledError::Measured(value) => value,
        SampledError::NoSamples => return Err(SolverError::InvalidPayload),
    };
    let report = regions.report()?;
    Ok((global, report.regions, report.grid_complete, report.root_work_charged))
}

fn peak_witnesses(
    errors: &[f64],
    references: &[f64],
    scratch: &[f64],
    samples: Layout,
    floor: f64,
    name: &'static str,
) -> Result<Witnesses, SolverError> {
    let points = samples.real_len();
    let error_peak = maximum(errors, points, samples)?;
    let relative_peak = relative_maximum(errors, references, points, samples, floor)?;
    let reference_peak = maximum(references, points, samples)?;
    let actual_peak = maximum(scratch, points, samples)?;
    let dimensions = samples.dimensions();
    let peak_distance = core::array::from_fn(|axis| {
        axis_distance(
            dimensions[axis],
            actual_peak.index[axis],
            reference_peak.index[axis],
        )
    });
    Ok(Witnesses {
        error_peak: FieldPeak::new(name, PeakSource::Error, error_peak),
        relative_peak: FieldPeak::new(name, PeakSource::RelativeError, relative_peak),
        reference_peak: FieldPeak::new(name, PeakSource::Reference, reference_peak),
        actual_peak: FieldPeak::new(name, PeakSource::Actual, actual_peak),
        peak_height_error: actual_peak.value - reference_peak.value,
        peak_distance,
    })
}

#[allow(clippy::too_many_arguments)]
fn sample_component(
    sampler: &mut DerivativeWorkspace,
    spectra: &[&[Complex64]],
    component: usize,
    orders: [u8; 3],
    quantity: PhysicalQuantity,
    index: usize,
    actual: &mut [f64],
    transforms: &mut usize,
) -> Result<(), SolverError> {
    let sampled = sampler.sample(spectra[component], Derivative::new(orders)?)?;
    actual.copy_from_slice(sampled.values);
    *transforms += 1;
    if quantity == PhysicalQuantity::Vorticity {
        let mut other = [0; 3];
        other[(index + 2) % 3] = 1;
        let curl = sampler.sample(spectra[(index + 1) % 3], Derivative::new(other)?)?;
        *transforms += 1;
        for (value, second) in actual.iter_mut().zip(curl.values) {
            *value -= *second;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn accumulate_points(
    actual: &[f64],
    errors: &mut [f64],
    references: &mut [f64],
    magnitudes: &mut [f64],
    reference: &ReferenceSide<'_>,
    quantity: PhysicalQuantity,
    index: usize,
) -> Result<(), SolverError> {
    for (point, (&actual, (error, (reference_magnitude, magnitude)))) in actual
        .iter()
        .zip(
            errors
                .iter_mut()
                .zip(references.iter_mut().zip(magnitudes.iter_mut())),
        )
        .enumerate()
    {
        let expected = reference.component(quantity, index, point);
        *error = hypot(*error, actual - expected)?;
        *reference_magnitude = hypot(*reference_magnitude, expected)?;
        *magnitude = hypot(*magnitude, actual)?;
    }
    Ok(())
}

fn hypot(current: f64, component: f64) -> Result<f64, SolverError> {
    let value = current.hypot(component);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SolverError::ArithmeticResolutionLimited)
    }
}

pub(crate) fn maximum(
    values: &[f64],
    points: usize,
    layout: Layout,
) -> Result<SampleMaximum, SolverError> {
    let mut found: Option<(usize, f64)> = None;
    for (linear, &value) in values.iter().take(points).enumerate() {
        if !value.is_finite() || value < 0.0 {
            return Err(SolverError::InvalidPayload);
        }
        if found.is_none_or(|(_, peak)| value > peak) {
            found = Some((linear, value));
        }
    }
    witness(found, layout, points)
}

fn relative_maximum(
    errors: &[f64],
    references: &[f64],
    points: usize,
    layout: Layout,
    floor: f64,
) -> Result<SampleMaximum, SolverError> {
    let mut found: Option<(usize, f64)> = None;
    for linear in 0..points {
        let value = errors[linear] / references[linear].max(floor);
        if !value.is_finite() {
            return Err(SolverError::InvalidPayload);
        }
        if found.is_none_or(|(_, peak)| value > peak) {
            found = Some((linear, value));
        }
    }
    witness(found, layout, points)
}

fn witness(
    found: Option<(usize, f64)>,
    layout: Layout,
    points: usize,
) -> Result<SampleMaximum, SolverError> {
    if points != layout.real_len() {
        return Err(SolverError::InvalidPayload);
    }
    let Some((linear, value)) = found else {
        return Err(SolverError::InvalidPayload);
    };
    let [_, ny, nz] = layout.dimensions();
    Ok(SampleMaximum {
        linear,
        index: [linear / (ny * nz), (linear / nz) % ny, linear % nz],
        value,
    })
}

impl FieldPeak {
    pub(crate) fn new(field: &'static str, source: PeakSource, maximum: SampleMaximum) -> Self {
        Self {
            field,
            source,
            maximum,
        }
    }

    pub(crate) fn identity(&self) -> String {
        format!("{}:{}", self.field, self.source.as_str())
    }
}
