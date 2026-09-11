//! Independent signed Fourier sums of full velocity, gradient, Hessian and curl differences.
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};

/// Independently accumulated sampled Euclidean/Frobenius error norms.
#[derive(Debug, Clone, Copy, Default)]
pub struct Norms {
    /// Root mean squared tensor magnitude over all physical grid points.
    pub rms: f64,
    /// Largest tensor magnitude on that same grid.
    pub peak: f64,
}

struct Mode {
    wave: [f64; 3],
    difference: [Complex64; 3],
}

#[derive(Default)]
struct Tensor {
    velocity: [f64; 3],
    gradient: [[f64; 3]; 3],
    hessian: [[[f64; 3]; 3]; 3],
}

fn coefficient(layout: Layout, field: &[Complex64], mode: [isize; 3]) -> Complex64 {
    let dimensions = layout.dimensions();
    if mode
        .iter()
        .zip(dimensions)
        .any(|(&m, n)| m.unsigned_abs() >= n / 2)
    {
        return Complex64::new(0.0, 0.0);
    }
    let conjugate = mode[2] < 0;
    let signed = mode.map(|m| if conjugate { -m } else { m });
    let position: [usize; 3] =
        std::array::from_fn(|axis| signed[axis].rem_euclid(dimensions[axis] as isize) as usize);
    let index = (position[0] * dimensions[1] + position[1]) * (dimensions[2] / 2 + 1) + position[2];
    if conjugate {
        field[index].conj()
    } else {
        field[index]
    }
}

fn modes(left: &SpectralState, right: &SpectralState) -> Vec<Mode> {
    let coarse = left.plan().domain().layout();
    let fine = right.plan().domain().layout();
    let [nx, ny, nz] = fine.dimensions().map(|n| n as isize / 2);
    let mut result = Vec::new();
    for x in -nx + 1..nx {
        for y in -ny + 1..ny {
            for z in -nz + 1..nz {
                let mode = [x, y, z];
                result.push(Mode {
                    wave: mode.map(|k| std::f64::consts::TAU * k as f64),
                    difference: std::array::from_fn(|axis| {
                        coefficient(fine, right.component(axis).unwrap(), mode)
                            - coefficient(coarse, left.component(axis).unwrap(), mode)
                    }),
                });
            }
        }
    }
    result
}

fn add_mode(tensor: &mut Tensor, mode: &Mode, point: [f64; 3]) {
    let phase = mode
        .wave
        .into_iter()
        .zip(point)
        .map(|(k, x)| k * x)
        .sum::<f64>();
    let (sin, cos) = phase.sin_cos();
    for component in 0..3 {
        let phased = mode.difference[component] * Complex64::new(cos, sin);
        tensor.velocity[component] += phased.re;
        for a in 0..3 {
            tensor.gradient[component][a] -= mode.wave[a] * phased.im;
            for b in 0..3 {
                tensor.hessian[component][a][b] -= mode.wave[a] * mode.wave[b] * phased.re;
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

/// All four complete tensor norms, with independently indexed modes and physical derivatives.
/// Input domains are the unit-cube family; every fine mode and the mean are retained.
pub fn pair_norms(left: &SpectralState, right: &SpectralState, samples: Layout) -> [Norms; 4] {
    let modes = modes(left, right);
    let [nx, ny, nz] = samples.dimensions();
    let mut sum = [0.0; 4];
    let mut peak = [0.0_f64; 4];
    for linear in 0..samples.real_len() {
        let point = [
            (linear / (ny * nz)) as f64 / nx as f64,
            ((linear / nz) % ny) as f64 / ny as f64,
            (linear % nz) as f64 / nz as f64,
        ];
        let mut tensor = Tensor::default();
        for mode in &modes {
            add_mode(&mut tensor, mode, point);
        }
        for (index, value) in squared(&tensor).into_iter().enumerate() {
            sum[index] += value;
            peak[index] = peak[index].max(value);
        }
    }
    std::array::from_fn(|index| Norms {
        rms: (sum[index] / samples.real_len() as f64).sqrt(),
        peak: peak[index].sqrt(),
    })
}
