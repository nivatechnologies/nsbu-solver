//! Independent signed Fourier reconstruction of actual velocity derivatives.
use nsbu_benchmarks::{fields::reference::ReferenceEvaluation, time::BenchmarkTime};
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};

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

#[derive(Default)]
struct Tensor {
    velocity: [f64; 3],
    gradient: [[f64; 3]; 3],
    hessian: [[[f64; 3]; 3]; 3],
}

fn coefficient(state: &SpectralState, component: usize, mode: [isize; 3]) -> Complex64 {
    let layout = state.plan().domain().layout();
    let dimensions = layout.dimensions();
    if mode
        .iter()
        .zip(dimensions)
        .any(|(&value, n)| value.unsigned_abs() >= n / 2)
    {
        return Complex64::new(0.0, 0.0);
    }
    let conjugate = mode[2] < 0;
    let signed = mode.map(|value| if conjugate { -value } else { value });
    let position: [usize; 3] =
        std::array::from_fn(|axis| signed[axis].rem_euclid(dimensions[axis] as isize) as usize);
    let index = (position[0] * dimensions[1] + position[1]) * (dimensions[2] / 2 + 1) + position[2];
    let value = state.component(component).unwrap()[index];
    if conjugate {
        value.conj()
    } else {
        value
    }
}

fn actual(state: &SpectralState, point: [f64; 3]) -> Tensor {
    let dimensions = state.plan().domain().layout().dimensions();
    let half = dimensions.map(|n| n as isize / 2);
    let mut tensor = Tensor::default();
    for x in -half[0] + 1..half[0] {
        for y in -half[1] + 1..half[1] {
            for z in -half[2] + 1..half[2] {
                add_mode(&mut tensor, state, [x, y, z], point);
            }
        }
    }
    tensor
}

fn add_mode(tensor: &mut Tensor, state: &SpectralState, mode: [isize; 3], point: [f64; 3]) {
    let wave = mode.map(|value| std::f64::consts::TAU * value as f64);
    let phase = wave
        .into_iter()
        .zip(point)
        .map(|(frequency, coordinate)| frequency * coordinate)
        .sum::<f64>();
    let (sin, cos) = phase.sin_cos();
    for component in 0..3 {
        let value = coefficient(state, component, mode) * Complex64::new(cos, sin);
        tensor.velocity[component] += value.re;
        for a in 0..3 {
            tensor.gradient[component][a] -= wave[a] * value.im;
            for b in 0..3 {
                tensor.hessian[component][a][b] -= wave[a] * wave[b] * value.re;
            }
        }
    }
}

fn fields(tensor: &Tensor, reference: ReferenceEvaluation) -> [(Vec<f64>, Vec<f64>); 4] {
    let curl = [
        tensor.gradient[2][1] - tensor.gradient[1][2],
        tensor.gradient[0][2] - tensor.gradient[2][0],
        tensor.gradient[1][0] - tensor.gradient[0][1],
    ];
    [
        (tensor.velocity.to_vec(), reference.velocity.to_vec()),
        (
            tensor.gradient.iter().flatten().copied().collect(),
            reference.gradient.iter().flatten().copied().collect(),
        ),
        (
            tensor.hessian.iter().flatten().flatten().copied().collect(),
            reference
                .hessian
                .iter()
                .flatten()
                .flatten()
                .copied()
                .collect(),
        ),
        (curl.to_vec(), reference.vorticity.to_vec()),
    ]
}

/// Compare reconstructed velocity derivatives with the manufactured reference.
pub fn tracking(
    state: &SpectralState,
    samples: Layout,
    time: BenchmarkTime,
    floors: [f64; 4],
) -> [Expected; 4] {
    let [nx, ny, nz] = samples.dimensions();
    let mut result = [Expected::default(); 4];
    for index in 0..samples.real_len() {
        let point = [
            (index / (ny * nz)) as f64 / nx as f64,
            ((index / nz) % ny) as f64 / ny as f64,
            (index % nz) as f64 / nz as f64,
        ];
        let reference = nsbu_benchmarks::fields::reference::evaluate(point, time).unwrap();
        for (quantity, (actual, reference)) in fields(&actual(state, point), reference)
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
