use nsbu_solver::{domain::Layout, Complex64};

use super::{
    core::{actual, coefficient, Tensor},
    phase,
};

#[cfg(test)]
fn legacy_actual(
    state: &nsbu_solver::domain::SpectralState,
    point: [f64; 3],
) -> (Tensor, Tensor, usize) {
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
/// cutoff-collar point, and a mixed-coordinate collar point. The scaled allowance is a
/// chosen floating-point comparison bound, not a rigorous certification of
/// the platform `libm` implementation.
pub fn assert_legacy_equivalence(state: &nsbu_solver::domain::SpectralState, samples: Layout) {
    let dimensions = state.plan().domain().layout().dimensions();
    assert!(matches!(dimensions, [4, 4, 4] | [8, 8, 8] | [12, 12, 12]));
    let basis = phase::Basis::new(dimensions, samples);
    let sample_dimensions = samples.dimensions();
    // Periodic-axis and two cutoff-collar points, one with mixed coordinates.
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
