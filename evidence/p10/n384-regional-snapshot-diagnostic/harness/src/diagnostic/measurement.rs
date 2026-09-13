use super::policy::{region_counts, region_index, region_names};
use super::{
    model, CoverageOutput, Quantity, QuantityReport, RegionCount, RegionOutput, SampleOutput,
    ROOT_BUDGET, STACK_BYTES, WORKERS,
};
use crate::cache::PackedReference;
use nsbu_benchmarks::{
    fields::reference,
    regions::{classify, CoveragePlan, CoverageStatus, NominalRegion},
    time::BenchmarkTime,
};
use nsbu_solver::{
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
    },
    domain::TickClock,
    Complex64,
};

pub(super) fn sample_reference(
    cache: &mut [PackedReference],
    labels: &mut [u8],
    header: model::ClockHeader,
    dimension: usize,
) -> Result<[RegionCount; 5], String> {
    if !dimension.is_multiple_of(WORKERS)
        || cache.len() != dimension.pow(3)
        || labels.len() != cache.len()
    {
        return Err("reference cache shape mismatch".into());
    }
    let clock = TickClock::restore(-20, 8192, header.elapsed, 8192 - header.elapsed)
        .map_err(model::debug)?;
    let time = BenchmarkTime::new(clock).map_err(model::debug)?;
    let plane = dimension * dimension;
    let planes_per_worker = dimension / WORKERS;
    let chunk = planes_per_worker * plane;
    let counts = std::thread::scope(|scope| -> Result<[usize; 5], String> {
        let mut handles = Vec::new();
        handles
            .try_reserve_exact(WORKERS)
            .map_err(|_| "worker handle allocation failed")?;
        for (worker, (cache_chunk, label_chunk)) in cache
            .chunks_mut(chunk)
            .zip(labels.chunks_mut(chunk))
            .enumerate()
        {
            let start_plane = worker * planes_per_worker;
            let handle = std::thread::Builder::new()
                .name(format!("regional-reference-{worker:02}"))
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, move || {
                    sample_reference_chunk(
                        cache_chunk,
                        label_chunk,
                        start_plane,
                        dimension,
                        clock,
                        time,
                    )
                })
                .map_err(model::debug)?;
            handles.push(handle);
        }
        let mut total = [0_usize; 5];
        for handle in handles {
            let local = handle.join().map_err(|_| "reference worker panicked")??;
            for (sum, value) in total.iter_mut().zip(local) {
                *sum = sum.checked_add(value).ok_or("region count overflow")?;
            }
        }
        Ok(total)
    })?;
    if counts.iter().sum::<usize>() != cache.len() {
        return Err("regional classification count mismatch".into());
    }
    Ok(region_counts(counts))
}

fn sample_reference_chunk(
    cache: &mut [PackedReference],
    labels: &mut [u8],
    start_plane: usize,
    dimension: usize,
    clock: TickClock,
    time: BenchmarkTime,
) -> Result<[usize; 5], String> {
    let plane = dimension * dimension;
    if cache.len() != labels.len() || !cache.len().is_multiple_of(plane) {
        return Err("reference worker chunk shape mismatch".into());
    }
    let mut counts = [0_usize; 5];
    for (local, (packed, label)) in cache.iter_mut().zip(labels).enumerate() {
        let i = start_plane + local / plane;
        let within = local % plane;
        let j = within / dimension;
        let k = within % dimension;
        let point = [
            i as f64 / dimension as f64,
            j as f64 / dimension as f64,
            k as f64 / dimension as f64,
        ];
        let independent = reference::evaluate(point, time).map_err(model::debug)?;
        *packed = PackedReference::pack_checked(&independent)?;
        let index = region_index(
            classify(point, clock, ROOT_BUDGET)
                .map_err(model::debug)?
                .spatial,
        );
        *label = u8::try_from(index).map_err(model::debug)?;
        counts[index] += 1;
    }
    Ok(counts)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn measure<const COMPONENTS: usize>(
    quantity: Quantity,
    coefficients: [&[Complex64]; 3],
    cache: &[PackedReference],
    labels: &[u8],
    workspace: &mut DerivativeWorkspace,
    errors: &mut [f64],
    references: &mut [f64],
) -> Result<QuantityReport, String> {
    if quantity.components() != COMPONENTS
        || cache.len() != errors.len()
        || cache.len() != references.len()
        || cache.len() != labels.len()
    {
        return Err("quantity workspace shape mismatch".into());
    }
    errors.fill(0.0);
    references.fill(0.0);
    for entry in 0..COMPONENTS {
        let (component, orders) = quantity.entry(entry);
        let actual = workspace
            .sample(
                coefficients[component],
                Derivative::new(orders).map_err(model::debug)?,
            )
            .map_err(model::debug)?;
        for (index, ((error, reference_magnitude), packed)) in errors
            .iter_mut()
            .zip(references.iter_mut())
            .zip(cache)
            .enumerate()
        {
            let expected = quantity.reference(*packed, entry);
            let difference = actual.values[index] - expected;
            *error = error.hypot(difference);
            *reference_magnitude = reference_magnitude.hypot(expected);
            if !error.is_finite() || !reference_magnitude.is_finite() {
                return Err("non-finite pointwise tensor magnitude".into());
            }
        }
    }
    let reference_peak = references.iter().copied().fold(0.0, f64::max);
    let relative_floor = quantity.absolute_floor().max(1e-3 * reference_peak);
    let (global, regions) = reduce::<COMPONENTS>(errors, references, labels, relative_floor)?;
    Ok(QuantityReport {
        quantity,
        scalar_inverse_transforms: quantity.transforms(),
        relative_floor,
        global,
        regions,
    })
}

pub(super) fn reduce<const COMPONENTS: usize>(
    errors: &[f64],
    references: &[f64],
    labels: &[u8],
    floor: f64,
) -> Result<(SampleOutput, [RegionOutput; 5]), String> {
    let mut global = TensorErrors::<COMPONENTS>::new(errors.len(), floor).map_err(model::debug)?;
    let empty = TensorErrors::<COMPONENTS>::new(errors.len(), floor).map_err(model::debug)?;
    let mut regional = [empty; 5];
    for ((&error, &reference), &label) in errors.iter().zip(references).zip(labels) {
        let index = usize::from(label);
        let region = regional
            .get_mut(index)
            .ok_or("invalid cached region label")?;
        global
            .push_magnitudes(error, reference)
            .map_err(model::debug)?;
        region
            .push_magnitudes(error, reference)
            .map_err(model::debug)?;
    }
    let mut outputs = region_names().map(|region| RegionOutput {
        region,
        sampled: SampleOutput::NoSamples,
    });
    for (output, accumulator) in outputs.iter_mut().zip(regional) {
        output.sampled = sample_output(accumulator.finish().map_err(model::debug)?);
    }
    Ok((
        sample_output(global.finish().map_err(model::debug)?),
        outputs,
    ))
}

fn sample_output(value: SampledError) -> SampleOutput {
    match value {
        SampledError::NoSamples => SampleOutput::NoSamples,
        SampledError::Measured(LocalError {
            components,
            samples,
            rms_error,
            peak_error,
            peak_relative_error,
            reference_peak,
            relative_floor,
        }) => SampleOutput::Measured {
            components,
            samples,
            rms_error,
            peak_error,
            peak_relative_error,
            reference_peak,
            relative_floor,
        },
    }
}

pub(super) fn coverage(clock: model::ClockHeader) -> Result<[CoverageOutput; 6], String> {
    let clock =
        TickClock::restore(-20, 8192, clock.elapsed, 8192 - clock.elapsed).map_err(model::debug)?;
    let mut output = [CoverageOutput {
        region: "",
        requested_panels: 0,
        fine_panels: 0,
        status: "",
        fraction: 0.0,
        refinement_change: 0.0,
        evaluations: 0,
    }; 6];
    let mut next = 0;
    for (name, region) in [
        ("core", NominalRegion::CORE),
        ("annulus", NominalRegion::ANNULUS),
    ] {
        for panels in [256, 512, 1024] {
            let measured = CoveragePlan::new(panels, 3 * panels + 2)
                .map_err(model::debug)?
                .evaluate(clock, region)
                .map_err(model::debug)?;
            output[next] = CoverageOutput {
                region: name,
                requested_panels: panels,
                fine_panels: measured.panels,
                status: match measured.status {
                    CoverageStatus::Nonempty => "nonempty",
                    CoverageStatus::RegionEmpty => "region_empty",
                },
                fraction: measured.fraction,
                refinement_change: measured.refinement_change,
                evaluations: measured.evaluations,
            };
            next += 1;
        }
    }
    Ok(output)
}
