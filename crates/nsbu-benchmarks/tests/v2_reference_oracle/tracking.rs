use nsbu_benchmarks::time::BenchmarkTime;
use nsbu_solver::{
    domain::{Domain, Layout, SpectralState},
    Complex64,
};

use super::core::{actual_view, fields};

#[derive(Debug, Clone, Copy, Default)]
/// Error summary for one reconstructed field quantity.
pub struct Expected {
    /// Root-mean-square error over the sampled grid.
    pub rms: f64,
    /// Largest absolute error over the sampled grid.
    pub peak: f64,
    /// Largest error relative to the reference magnitude and configured floor.
    pub relative_peak: f64,
    /// Largest reference magnitude over the sampled grid.
    pub reference_peak: f64,
}

/// Compare reconstructed velocity derivatives with the manufactured reference.
pub fn tracking(
    state: &SpectralState,
    samples: Layout,
    time: BenchmarkTime,
    floors: [f64; 4],
) -> [Expected; 4] {
    tracking_view(
        state.plan().domain(),
        [
            state.component(0).unwrap(),
            state.component(1).unwrap(),
            state.component(2).unwrap(),
        ],
        samples,
        time,
        floors,
    )
}

/// Compare borrowed reconstructed velocity coefficients with the manufactured reference.
pub fn tracking_view(
    domain: Domain,
    values: [&[Complex64]; 3],
    samples: Layout,
    time: BenchmarkTime,
    floors: [f64; 4],
) -> [Expected; 4] {
    let [_, ny, nz] = samples.dimensions();
    let basis = super::phase::Basis::new(domain.layout().dimensions(), samples);
    let mut result = [Expected::default(); 4];
    for index in 0..samples.real_len() {
        let sample_index = [index / (ny * nz), (index / nz) % ny, index % nz];
        let point = basis.point(sample_index);
        let reference = nsbu_benchmarks::fields::reference::evaluate(point, time).unwrap();
        for (quantity, (actual, reference)) in fields(
            &actual_view(domain, values, sample_index, &basis),
            reference,
        )
        .into_iter()
        .enumerate()
        {
            accumulate(&mut result[quantity], &actual, &reference, floors[quantity]);
        }
    }
    for value in &mut result {
        value.rms = (value.rms / samples.real_len() as f64).sqrt();
    }
    result
}

fn accumulate(value: &mut Expected, actual: &[f64], reference: &[f64], floor: f64) {
    let error = actual
        .iter()
        .zip(reference)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt();
    let scale = reference.iter().map(|v| v * v).sum::<f64>().sqrt();
    value.rms += error * error;
    value.peak = value.peak.max(error);
    value.relative_peak = value.relative_peak.max(error / scale.max(floor));
    value.reference_peak = value.reference_peak.max(scale);
}
