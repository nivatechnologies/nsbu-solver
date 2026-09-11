use nsbu_benchmarks::time::BenchmarkTime;
use nsbu_solver::{domain::Layout, domain::SpectralState};

use super::core::{actual, fields};

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
    let [_, ny, nz] = samples.dimensions();
    let basis = super::phase::Basis::new(state.plan().domain().layout().dimensions(), samples);
    let mut result = [Expected::default(); 4];
    for index in 0..samples.real_len() {
        let sample_index = [index / (ny * nz), (index / nz) % ny, index % nz];
        let point = basis.point(sample_index);
        let reference = nsbu_benchmarks::fields::reference::evaluate(point, time).unwrap();
        for (quantity, (actual, reference)) in
            fields(&actual(state, sample_index, &basis), reference)
                .into_iter()
                .enumerate()
        {
            let error = actual
                .iter()
                .zip(&reference)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            let scale = reference
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            result[quantity].rms += error * error;
            result[quantity].peak = result[quantity].peak.max(error);
            result[quantity].relative_peak = result[quantity]
                .relative_peak
                .max(error / scale.max(floors[quantity]));
            result[quantity].reference_peak = result[quantity].reference_peak.max(scale);
        }
    }
    for value in &mut result {
        value.rms = (value.rms / samples.real_len() as f64).sqrt();
    }
    result
}
