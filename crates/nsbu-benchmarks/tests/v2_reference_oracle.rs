//! Independent signed Fourier reconstruction of actual velocity derivatives.
use nsbu_benchmarks::{fields::reference::ReferenceEvaluation, time::BenchmarkTime};
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};
#[path = "v2_reference_oracle/phase.rs"]
mod phase;

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

fn actual(state: &SpectralState, point: [usize; 3], basis: &phase::Basis) -> Tensor {
    let dimensions = state.plan().domain().layout().dimensions();
    let half = dimensions.map(|n| n as isize / 2);
    let mut tensor = Tensor::default();
    for x in -half[0] + 1..half[0] {
        for y in -half[1] + 1..half[1] {
            for z in -half[2] + 1..half[2] {
                add_mode(&mut tensor, state, [x, y, z], basis.phase([x, y, z], point));
            }
        }
    }
    tensor
}

fn add_mode(tensor: &mut Tensor, state: &SpectralState, mode: [isize; 3], phase: Complex64) {
    let wave = mode.map(|value| std::f64::consts::TAU * value as f64);
    for component in 0..3 {
        let value = coefficient(state, component, mode) * phase;
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
    let [_, ny, nz] = samples.dimensions();
    let basis = phase::Basis::new(state.plan().domain().layout().dimensions(), samples);
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

#[cfg(test)]
fn legacy_actual(state: &SpectralState, point: [f64; 3]) -> (Tensor, Tensor, usize) {
    let dimensions = state.plan().domain().layout().dimensions();
    let half = dimensions.map(|n| n as isize / 2);
    let mut tensor = Tensor::default();
    let mut magnitudes = Tensor::default();
    let mut terms = 0;
    for x in -half[0] + 1..half[0] {
        for y in -half[1] + 1..half[1] {
            for z in -half[2] + 1..half[2] {
                let mode = [x, y, z];
                let wave = mode.map(|value| std::f64::consts::TAU * value as f64);
                let angle = wave.into_iter().zip(point).map(|(k, p)| k * p).sum::<f64>();
                let (sin, cos) = angle.sin_cos();
                for component in 0..3 {
                    let coefficient = coefficient(state, component, mode);
                    let value = coefficient * Complex64::new(cos, sin);
                    let amplitude = coefficient.re.abs() + coefficient.im.abs();
                    tensor.velocity[component] += value.re;
                    magnitudes.velocity[component] += amplitude;
                    for a in 0..3 {
                        tensor.gradient[component][a] -= wave[a] * value.im;
                        magnitudes.gradient[component][a] += wave[a].abs() * amplitude;
                        for b in 0..3 {
                            tensor.hessian[component][a][b] -= wave[a] * wave[b] * value.re;
                            magnitudes.hessian[component][a][b] +=
                                (wave[a] * wave[b]).abs() * amplitude;
                        }
                    }
                }
                terms += 1;
            }
        }
    }
    (tensor, magnitudes, terms)
}

#[cfg(test)]
fn assert_component(actual: f64, legacy: f64, magnitude: f64, terms: usize) {
    if magnitude == 0.0 {
        assert_eq!(actual.to_bits(), legacy.to_bits());
        return;
    }
    let allowance = 128.0 * f64::EPSILON * terms as f64 * magnitude;
    assert!(
        (actual - legacy).abs() <= allowance,
        "cached={actual:e} legacy={legacy:e} magnitude={magnitude:e} terms={terms} allowance={allowance:e}"
    );
}

#[cfg(test)]
fn assert_phases(
    dimensions: [usize; 3],
    basis: &phase::Basis,
    point_index: [usize; 3],
    point: [f64; 3],
) {
    let half = dimensions.map(|n| n as isize / 2);
    let mut saw_negative_z = false;
    for x in -half[0] + 1..half[0] {
        for y in -half[1] + 1..half[1] {
            for z in -half[2] + 1..half[2] {
                let mode = [x, y, z];
                saw_negative_z |= z < 0;
                let wave = mode.map(|value| std::f64::consts::TAU * value as f64);
                let angle = wave.into_iter().zip(point).map(|(k, p)| k * p).sum::<f64>();
                let (sin, cos) = angle.sin_cos();
                let legacy = Complex64::new(cos, sin);
                let cached = basis.phase(mode, point_index);
                assert!((cached - legacy).l1_norm() <= 32.0 * f64::EPSILON);
            }
        }
    }
    assert!(saw_negative_z);
}

#[cfg(test)]
fn curl(tensor: &Tensor) -> [f64; 3] {
    [
        tensor.gradient[2][1] - tensor.gradient[1][2],
        tensor.gradient[0][2] - tensor.gradient[2][0],
        tensor.gradient[1][0] - tensor.gradient[0][1],
    ]
}

#[cfg(test)]
fn assert_tensors(cached: &Tensor, legacy: &Tensor, magnitudes: &Tensor, terms: usize) {
    for component in 0..3 {
        assert_component(
            cached.velocity[component],
            legacy.velocity[component],
            magnitudes.velocity[component],
            terms,
        );
        for a in 0..3 {
            assert_component(
                cached.gradient[component][a],
                legacy.gradient[component][a],
                magnitudes.gradient[component][a],
                terms,
            );
            for b in 0..3 {
                assert_component(
                    cached.hessian[component][a][b],
                    legacy.hessian[component][a][b],
                    magnitudes.hessian[component][a][b],
                    terms,
                );
            }
        }
    }
    let magnitude = magnitudes.gradient.iter().flatten().sum::<f64>();
    for (cached, legacy) in curl(cached).into_iter().zip(curl(legacy)) {
        assert_component(cached, legacy, magnitude, terms);
    }
}

#[cfg(test)]
/// Compares the cached phase basis with the legacy direct-angle oracle.
///
/// This focused check covers every signed mode at an axis point, a sampled
/// cutoff-collar point, and an interior point. The scaled allowance is a
/// chosen floating-point comparison bound, not a rigorous certification of
/// the platform `libm` implementation.
pub fn assert_legacy_equivalence(state: &SpectralState, samples: Layout) {
    let dimensions = state.plan().domain().layout().dimensions();
    assert!(matches!(dimensions, [4, 4, 4] | [8, 8, 8] | [12, 12, 12]));
    let basis = phase::Basis::new(dimensions, samples);
    let sample_dimensions = samples.dimensions();
    // Periodic-axis, sampled cutoff-collar, and interior points.
    for point_index in [
        [0, 0, 1],
        [5.min(sample_dimensions[0] - 1), 0, 0],
        [1, 2, 3],
    ] {
        let point = basis.point(point_index);
        assert_phases(dimensions, &basis, point_index, point);
        let cached = actual(state, point_index, &basis);
        let (legacy, magnitudes, terms) = legacy_actual(state, point);
        assert_tensors(&cached, &legacy, &magnitudes, terms);
    }
}
