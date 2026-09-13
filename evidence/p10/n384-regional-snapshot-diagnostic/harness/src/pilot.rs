use crate::cache::{PackedReference, WORDS};
use nsbu_benchmarks::{
    fields::reference,
    regions::{classify, SpatialRegion},
    time::BenchmarkTime,
};
use nsbu_solver::domain::TickClock;
use serde::Serialize;
use std::time::Instant;

const DIMENSION: usize = 768;
const X_INDEX: usize = 0;
const ROOT_BUDGET: usize = 128;

#[derive(Debug, Serialize)]
pub(crate) struct PilotOutput {
    schema: &'static str,
    status: &'static str,
    scope: &'static str,
    dimensions: [usize; 3],
    x_index: usize,
    x_coordinate: f64,
    points: usize,
    packed_words_per_point: usize,
    packed_bytes: usize,
    ordered_hessian_comparisons: usize,
    reference_seconds: f64,
    classification_seconds: f64,
    total_seconds: f64,
    sequential_full_grid_extrapolation_seconds: f64,
    ideal_32_worker_reference_extrapolation_seconds: f64,
    region_counts: RegionCounts,
    representative_coordinates: RepresentativeCoordinates,
    collar_volume_coverage: &'static str,
    peak_qualification: &'static str,
    acceptance: Acceptance,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct RegionCounts {
    core: usize,
    annulus: usize,
    interior_outside_nominal: usize,
    collar: usize,
    exterior: usize,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct RepresentativeCoordinates {
    core: Option<[f64; 3]>,
    annulus: Option<[f64; 3]>,
    interior_outside_nominal: Option<[f64; 3]>,
    collar: Option<[f64; 3]>,
    exterior: Option<[f64; 3]>,
}

#[derive(Debug, Serialize)]
struct Acceptance {
    status: &'static str,
    accepted_windows: usize,
}

pub(crate) fn run() -> Result<PilotOutput, String> {
    let clock = TickClock::restore(-20, 8192, 512, 7680).map_err(crate::model::debug)?;
    let time = BenchmarkTime::new(clock).map_err(crate::model::debug)?;
    let points = DIMENSION
        .checked_mul(DIMENSION)
        .ok_or("pilot point count overflow")?;
    let mut cache = try_zeros(points)?;
    let total_started = Instant::now();
    let reference_started = Instant::now();
    for j in 0..DIMENSION {
        for k in 0..DIMENSION {
            let point = coordinate(j, k);
            let independent = reference::evaluate(point, time).map_err(crate::model::debug)?;
            cache[j * DIMENSION + k] = PackedReference::pack_checked(&independent)?;
        }
    }
    let reference_seconds = reference_started.elapsed().as_secs_f64();
    let classification_started = Instant::now();
    let mut region_counts = RegionCounts::default();
    let mut representative_coordinates = RepresentativeCoordinates::default();
    for j in 0..DIMENSION {
        for k in 0..DIMENSION {
            let point = coordinate(j, k);
            record(
                classify(point, clock, ROOT_BUDGET)
                    .map_err(crate::model::debug)?
                    .spatial,
                point,
                &mut region_counts,
                &mut representative_coordinates,
            );
        }
    }
    let classification_seconds = classification_started.elapsed().as_secs_f64();
    let total_seconds = total_started.elapsed().as_secs_f64();
    let interior = region_counts
        .core
        .checked_add(region_counts.annulus)
        .and_then(|value| value.checked_add(region_counts.interior_outside_nominal))
        .ok_or("pilot count overflow")?;
    if interior == 0 || region_counts.collar == 0 || region_counts.exterior == 0 {
        return Err(
            "pilot plane failed to cover interior, collar and exterior cutoff coordinates".into(),
        );
    }
    let plane_multiple = DIMENSION as f64;
    Ok(PilotOutput {
        schema: "p10-n384-regional-x-plane-pilot-v1",
        status: "representative-full-x-plane-complete",
        scope: "binary64 timing and mapping pilot only; no snapshot load or full-grid claim",
        dimensions: [DIMENSION; 3],
        x_index: X_INDEX,
        x_coordinate: X_INDEX as f64 / DIMENSION as f64,
        points,
        packed_words_per_point: WORDS,
        packed_bytes: points * size_of::<PackedReference>(),
        ordered_hessian_comparisons: points * 27,
        reference_seconds,
        classification_seconds,
        total_seconds,
        sequential_full_grid_extrapolation_seconds: total_seconds * plane_multiple,
        ideal_32_worker_reference_extrapolation_seconds: reference_seconds * plane_multiple / 32.0,
        region_counts,
        representative_coordinates,
        collar_volume_coverage: "not_assessed_sampled_collar_counts_only",
        peak_qualification: "not_assessed",
        acceptance: Acceptance {
            status: "not_assessed",
            accepted_windows: 0,
        },
    })
}

fn coordinate(j: usize, k: usize) -> [f64; 3] {
    [
        X_INDEX as f64 / DIMENSION as f64,
        j as f64 / DIMENSION as f64,
        k as f64 / DIMENSION as f64,
    ]
}

fn record(
    region: SpatialRegion,
    point: [f64; 3],
    counts: &mut RegionCounts,
    coordinates: &mut RepresentativeCoordinates,
) {
    let (count, coordinate) = match region {
        SpatialRegion::Core => (&mut counts.core, &mut coordinates.core),
        SpatialRegion::Annulus => (&mut counts.annulus, &mut coordinates.annulus),
        SpatialRegion::InteriorOutsideNominal => (
            &mut counts.interior_outside_nominal,
            &mut coordinates.interior_outside_nominal,
        ),
        SpatialRegion::Collar => (&mut counts.collar, &mut coordinates.collar),
        SpatialRegion::Exterior => (&mut counts.exterior, &mut coordinates.exterior),
    };
    *count += 1;
    coordinate.get_or_insert(point);
}

fn try_zeros<T: Clone + Default>(length: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "bounded pilot allocation failed")?;
    values.resize(length, T::default());
    Ok(values)
}
