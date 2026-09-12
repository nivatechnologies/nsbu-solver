use crate::{
    fields::reference::{self, ReferenceEvaluation},
    time::BenchmarkTime,
};
use nsbu_solver::{
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
        physical::PhysicalQuantity,
    },
    domain::{Domain, Layout, TickClock},
    Complex64, SolverError,
};

pub(crate) fn derivatives(
    sources: [Domain; 3],
    samples: Layout,
    cap: usize,
) -> Result<[DerivativeWorkspace; 3], SolverError> {
    Ok([
        DerivativeWorkspace::new(sources[0], samples, cap)?,
        DerivativeWorkspace::new(sources[1], samples, cap)?,
        DerivativeWorkspace::new(sources[2], samples, cap)?,
    ])
}

pub(crate) fn real(count: usize) -> Result<Vec<f64>, SolverError> {
    filled(count, 0.0)
}

pub(crate) fn references(count: usize) -> Result<Vec<ReferenceEvaluation>, SolverError> {
    filled(count, zero_reference())
}

pub(crate) fn evaluate_references(
    references: &mut [ReferenceEvaluation],
    samples: Layout,
    clock: TickClock,
) -> Result<(), super::ReferenceTrackingError> {
    let time = BenchmarkTime::new(clock)?;
    let [nx, ny, nz] = samples.dimensions();
    for (index, output) in references.iter_mut().enumerate() {
        let point = [
            (index / (ny * nz)) as f64 / nx as f64,
            ((index / nz) % ny) as f64 / ny as f64,
            (index % nz) as f64 / nz as f64,
        ];
        *output = reference::evaluate(point, time)?;
    }
    Ok(())
}

/// Temporary borrowed view of one tracking owner's preallocated numerical scratch.
pub(crate) struct TrackingScratch<'a> {
    /// Three retained-resolution derivative streams.
    pub(crate) derivatives: &'a mut [DerivativeWorkspace; 3],
    /// Current scalar component sampled on the physical lattice.
    pub(crate) actual: &'a mut [f64],
    /// Pointwise accumulated tensor error magnitudes.
    pub(crate) errors: &'a mut [f64],
    /// Pointwise accumulated analytical tensor magnitudes.
    pub(crate) reference_magnitudes: &'a mut [f64],
    /// Analytical values cached once for the current clock and lattice.
    pub(crate) references: &'a [ReferenceEvaluation],
}

impl TrackingScratch<'_> {
    /// Fill error/reference magnitudes for one complete physical quantity.
    pub(crate) fn prepare_quantity(
        &mut self,
        values: [&[Complex64]; 3],
        branch: usize,
        quantity: PhysicalQuantity,
    ) -> Result<(), SolverError> {
        self.errors.fill(0.0);
        self.reference_magnitudes.fill(0.0);
        for component in 0..quantity.components() {
            sample_component(
                self.derivatives,
                self.actual,
                values,
                branch,
                quantity,
                component,
            )?;
            accumulate_points(
                self.actual,
                self.errors,
                self.reference_magnitudes,
                self.references,
                quantity,
                component,
            )?;
        }
        Ok(())
    }
}

fn sample_component(
    derivatives: &mut [DerivativeWorkspace; 3],
    actual: &mut [f64],
    values: [&[Complex64]; 3],
    branch: usize,
    quantity: PhysicalQuantity,
    index: usize,
) -> Result<(), SolverError> {
    let workspace = if branch < 2 { branch } else { 2 };
    let (component, derivative) = entry(quantity, index);
    actual.copy_from_slice(
        derivatives[workspace]
            .sample(values[component], Derivative::new(derivative)?)?
            .values,
    );
    if quantity == PhysicalQuantity::Vorticity {
        subtract_curl(derivatives, actual, values, workspace, index)?;
    }
    Ok(())
}

fn subtract_curl(
    derivatives: &mut [DerivativeWorkspace; 3],
    actual: &mut [f64],
    values: [&[Complex64]; 3],
    workspace: usize,
    index: usize,
) -> Result<(), SolverError> {
    let mut other = [0; 3];
    other[(index + 2) % 3] = 1;
    let sampled =
        derivatives[workspace].sample(values[(index + 1) % 3], Derivative::new(other)?)?;
    for (actual, &second) in actual.iter_mut().zip(sampled.values) {
        *actual -= second;
    }
    Ok(())
}

fn accumulate_points(
    actual: &[f64],
    errors: &mut [f64],
    reference_magnitudes: &mut [f64],
    references: &[ReferenceEvaluation],
    quantity: PhysicalQuantity,
    component: usize,
) -> Result<(), SolverError> {
    for index in 0..actual.len() {
        let expected = reference_component(references[index], quantity, component);
        errors[index] = accumulate_magnitude(errors[index], actual[index] - expected)?;
        reference_magnitudes[index] = accumulate_magnitude(reference_magnitudes[index], expected)?;
    }
    Ok(())
}

pub(crate) fn reduce(
    components: usize,
    errors: &[f64],
    references: &[f64],
    floor: f64,
) -> Result<LocalError, SolverError> {
    match components {
        3 => reduce_typed::<3>(errors, references, floor),
        9 => reduce_typed::<9>(errors, references, floor),
        27 => reduce_typed::<27>(errors, references, floor),
        _ => Err(SolverError::InvalidPayload),
    }
}

fn reduce_typed<const C: usize>(
    errors: &[f64],
    references: &[f64],
    floor: f64,
) -> Result<LocalError, SolverError> {
    let mut result = TensorErrors::<C>::new(errors.len(), floor)?;
    for (&error, &reference) in errors.iter().zip(references) {
        result.push_magnitudes(error, reference)?;
    }
    match result.finish()? {
        SampledError::Measured(value) => Ok(value),
        SampledError::NoSamples => Err(SolverError::InvalidPayload),
    }
}

fn entry(quantity: PhysicalQuantity, index: usize) -> (usize, [u8; 3]) {
    let mut orders = [0; 3];
    let component = match quantity {
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
        _ => 0,
    };
    (component, orders)
}

fn reference_component(
    value: ReferenceEvaluation,
    quantity: PhysicalQuantity,
    index: usize,
) -> f64 {
    match quantity {
        PhysicalQuantity::Vector => value.velocity[index],
        PhysicalQuantity::Gradient => value.gradient[index / 3][index % 3],
        PhysicalQuantity::Hessian => value.hessian[index / 9][(index / 3) % 3][index % 3],
        PhysicalQuantity::Vorticity => value.vorticity[index],
        _ => f64::NAN,
    }
}

pub(super) fn accumulate_magnitude(current: f64, component: f64) -> Result<f64, SolverError> {
    let result = current.hypot(component);
    if result.is_finite() {
        Ok(result)
    } else {
        Err(SolverError::InvalidSpectrum)
    }
}

fn zero_reference() -> ReferenceEvaluation {
    ReferenceEvaluation {
        velocity: [0.0; 3],
        gradient: [[0.0; 3]; 3],
        hessian: [[[0.0; 3]; 3]; 3],
        vorticity: [0.0; 3],
        pressure_raw: 0.0,
        pressure_gradient: [0.0; 3],
        root: None,
    }
}

fn filled<T: Clone>(count: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(count, value);
    Ok(values)
}
