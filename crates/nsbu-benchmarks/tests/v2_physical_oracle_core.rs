//! Independent signed Fourier sums of full velocity, gradient, Hessian and curl differences.
use nsbu_solver::{
    domain::{Domain, Layout},
    Complex64,
};

/// Complete independently reduced pointwise statistics and first peak locations.
#[derive(Debug, Clone, Copy, Default)]
pub struct Errors {
    /// Root mean squared tensor error over sample points.
    pub rms: f64,
    /// Largest pointwise tensor error.
    pub peak: f64,
    /// Largest pointwise error divided by the floored reference magnitude.
    pub relative_peak: f64,
    /// Largest pointwise reference tensor magnitude.
    pub reference_peak: f64,
    /// First independently observed absolute-error peak linear index.
    pub peak_linear: usize,
    /// First independently observed relative-error peak linear index.
    pub relative_linear: usize,
    /// First independently observed reference peak linear index.
    pub reference_linear: usize,
}

struct Mode {
    wave: [f64; 3],
    difference: [Complex64; 3],
    reference: [Complex64; 3],
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

fn view_modes(
    left_domain: Domain,
    left: [&[Complex64]; 3],
    right_domain: Domain,
    right: [&[Complex64]; 3],
) -> Vec<Mode> {
    let coarse = left_domain.layout();
    let fine = right_domain.layout();
    let [nx, ny, nz] = fine.dimensions().map(|n| n as isize / 2);
    let mut result = Vec::new();
    for x in -nx + 1..nx {
        for y in -ny + 1..ny {
            for z in -nz + 1..nz {
                let mode = [x, y, z];
                let reference = std::array::from_fn(|axis| coefficient(fine, right[axis], mode));
                result.push(Mode {
                    wave: mode.map(|k| std::f64::consts::TAU * k as f64),
                    difference: std::array::from_fn(|axis| {
                        reference[axis] - coefficient(coarse, left[axis], mode)
                    }),
                    reference,
                });
            }
        }
    }
    result
}

fn add_mode(tensor: &mut Tensor, values: [Complex64; 3], mode: &Mode, point: [f64; 3]) {
    let phase = mode
        .wave
        .into_iter()
        .zip(point)
        .map(|(k, x)| k * x)
        .sum::<f64>();
    let (sin, cos) = phase.sin_cos();
    for (component, coefficient) in values.into_iter().enumerate() {
        let phased = coefficient * Complex64::new(cos, sin);
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

fn at_point(modes: &[Mode], samples: Layout, linear: usize) -> ([f64; 4], [f64; 4]) {
    let [nx, ny, nz] = samples.dimensions();
    let point = [
        (linear / (ny * nz)) as f64 / nx as f64,
        ((linear / nz) % ny) as f64 / ny as f64,
        (linear % nz) as f64 / nz as f64,
    ];
    let mut difference = Tensor::default();
    let mut reference = Tensor::default();
    for mode in modes {
        add_mode(&mut difference, mode.difference, mode, point);
        add_mode(&mut reference, mode.reference, mode, point);
    }
    (
        squared(&difference).map(f64::sqrt),
        squared(&reference).map(f64::sqrt),
    )
}

fn reduce(modes: Vec<Mode>, samples: Layout, floors: [f64; 4]) -> [Errors; 4] {
    let mut sum = [0.0; 4];
    let mut result = [Errors::default(); 4];
    for linear in 0..samples.real_len() {
        let (errors, references) = at_point(&modes, samples, linear);
        for index in 0..4 {
            sum[index] += errors[index] * errors[index];
            let relative = errors[index] / references[index].max(floors[index]);
            if linear == 0 || errors[index] > result[index].peak {
                result[index].peak = errors[index];
                result[index].peak_linear = linear;
            }
            if linear == 0 || relative > result[index].relative_peak {
                result[index].relative_peak = relative;
                result[index].relative_linear = linear;
            }
            if linear == 0 || references[index] > result[index].reference_peak {
                result[index].reference_peak = references[index];
                result[index].reference_linear = linear;
            }
        }
    }
    for index in 0..4 {
        result[index].rms = (sum[index] / samples.real_len() as f64).sqrt();
    }
    result
}

/// All reductions for borrowed coefficient components and their explicit domains.
pub fn view_errors(
    left_domain: Domain,
    left: [&[Complex64]; 3],
    right_domain: Domain,
    right: [&[Complex64]; 3],
    samples: Layout,
    floors: [f64; 4],
) -> [Errors; 4] {
    reduce(
        view_modes(left_domain, left, right_domain, right),
        samples,
        floors,
    )
}

/// Point magnitudes for borrowed coefficient components and their explicit domains.
pub fn view_point_magnitudes(
    left_domain: Domain,
    left: [&[Complex64]; 3],
    right_domain: Domain,
    right: [&[Complex64]; 3],
    samples: Layout,
    linear: usize,
) -> ([f64; 4], [f64; 4]) {
    assert!(linear < samples.real_len());
    at_point(
        &view_modes(left_domain, left, right_domain, right),
        samples,
        linear,
    )
}
