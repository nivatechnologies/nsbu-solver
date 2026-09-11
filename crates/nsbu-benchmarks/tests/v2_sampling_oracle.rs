//! Independent signed Fourier sums at selected physical sample indices.
#[path = "v2_sampling_oracle/modes.rs"]
mod modes;
use modes::{modes, Tensor};
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};

pub(crate) fn retained_band_excludes(state: &SpectralState, mode: [isize; 3]) -> bool {
    modes::coefficient(state, mode, 0) == Complex64::new(0.0, 0.0)
}

fn add(tensor: &mut Tensor, values: [Complex64; 3], wave: [f64; 3], phase: Complex64) {
    for (component, coefficient) in values.into_iter().enumerate() {
        let value = coefficient * phase;
        tensor.velocity[component] += value.re;
        for a in 0..3 {
            tensor.gradient[component][a] -= wave[a] * value.im;
            for b in 0..3 {
                tensor.hessian[component][a][b] -= wave[a] * wave[b] * value.re;
            }
        }
    }
}

fn squared(tensor: &Tensor) -> [f64; 4] {
    let curl = [
        tensor.gradient[2][1] - tensor.gradient[1][2],
        tensor.gradient[0][2] - tensor.gradient[2][0],
        tensor.gradient[1][0] - tensor.gradient[0][1],
    ];
    [
        tensor.velocity.iter().map(|x| x * x).sum(),
        tensor.gradient.iter().flatten().map(|x| x * x).sum(),
        tensor
            .hessian
            .iter()
            .flatten()
            .flatten()
            .map(|x| x * x)
            .sum(),
        curl.iter().map(|x| x * x).sum(),
    ]
}

/// Error and finer-state magnitudes for all four quantities at one exact sample index.
pub fn magnitudes(
    left: &SpectralState,
    right: &SpectralState,
    samples: Layout,
    index: [usize; 3],
) -> ([f64; 4], [f64; 4]) {
    let dimensions = samples.dimensions();
    assert!(index.into_iter().zip(dimensions).all(|(i, n)| i < n));
    let point: [f64; 3] = std::array::from_fn(|axis| index[axis] as f64 / dimensions[axis] as f64);
    let mut coarse = Tensor::default();
    let mut fine = Tensor::default();
    for mode in modes(left, right) {
        let angle = mode
            .wave
            .into_iter()
            .zip(point)
            .map(|(k, x)| k * x)
            .sum::<f64>();
        let (sin, cos) = angle.sin_cos();
        let phase = Complex64::new(cos, sin);
        add(&mut coarse, mode.left, mode.wave, phase);
        add(&mut fine, mode.right, mode.wave, phase);
    }
    let difference = Tensor {
        velocity: std::array::from_fn(|i| coarse.velocity[i] - fine.velocity[i]),
        gradient: std::array::from_fn(|i| {
            std::array::from_fn(|a| coarse.gradient[i][a] - fine.gradient[i][a])
        }),
        hessian: std::array::from_fn(|i| {
            std::array::from_fn(|a| {
                std::array::from_fn(|b| coarse.hessian[i][a][b] - fine.hessian[i][a][b])
            })
        }),
    };
    (
        squared(&difference).map(f64::sqrt),
        squared(&fine).map(f64::sqrt),
    )
}
