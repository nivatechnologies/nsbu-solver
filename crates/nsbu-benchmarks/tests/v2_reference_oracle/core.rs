use nsbu_benchmarks::fields::reference::ReferenceEvaluation;
use nsbu_solver::{
    domain::{Domain, SpectralState},
    Complex64,
};

#[derive(Default)]
pub(super) struct Tensor {
    pub(super) velocity: [f64; 3],
    pub(super) gradient: [[f64; 3]; 3],
    pub(super) hessian: [[[f64; 3]; 3]; 3],
}

pub(super) fn coefficient(state: &SpectralState, component: usize, mode: [isize; 3]) -> Complex64 {
    coefficient_view(
        state.plan().domain(),
        [
            state.component(0).unwrap(),
            state.component(1).unwrap(),
            state.component(2).unwrap(),
        ],
        component,
        mode,
    )
}

fn coefficient_view(
    domain: Domain,
    values: [&[Complex64]; 3],
    component: usize,
    mode: [isize; 3],
) -> Complex64 {
    let layout = domain.layout();
    let dimensions = layout.dimensions();
    if outside_strict_band(mode, dimensions) {
        return Complex64::new(0.0, 0.0);
    }
    let conjugate = mode[2] < 0;
    let signed = orient(mode, conjugate);
    let position: [usize; 3] =
        std::array::from_fn(|axis| signed[axis].rem_euclid(dimensions[axis] as isize) as usize);
    let index = (position[0] * dimensions[1] + position[1]) * (dimensions[2] / 2 + 1) + position[2];
    let value = values[component][index];
    if conjugate {
        value.conj()
    } else {
        value
    }
}

fn outside_strict_band(mode: [isize; 3], dimensions: [usize; 3]) -> bool {
    mode.iter()
        .zip(dimensions)
        .any(|(&value, n)| value.unsigned_abs() >= n / 2)
}

fn orient(mode: [isize; 3], conjugate: bool) -> [isize; 3] {
    mode.map(|value| if conjugate { -value } else { value })
}

pub(super) fn actual(
    state: &SpectralState,
    point: [usize; 3],
    basis: &super::phase::Basis,
) -> Tensor {
    actual_view(
        state.plan().domain(),
        [
            state.component(0).unwrap(),
            state.component(1).unwrap(),
            state.component(2).unwrap(),
        ],
        point,
        basis,
    )
}

pub(super) fn actual_view(
    domain: Domain,
    values: [&[Complex64]; 3],
    point: [usize; 3],
    basis: &super::phase::Basis,
) -> Tensor {
    let dimensions = domain.layout().dimensions();
    let half = dimensions.map(|n| n as isize / 2);
    let mut tensor = Tensor::default();
    for x in -half[0] + 1..half[0] {
        for y in -half[1] + 1..half[1] {
            add_line(&mut tensor, domain, values, [x, y], half[2], point, basis);
        }
    }
    tensor
}

fn add_line(
    tensor: &mut Tensor,
    domain: Domain,
    values: [&[Complex64]; 3],
    transverse: [isize; 2],
    half_z: isize,
    point: [usize; 3],
    basis: &super::phase::Basis,
) {
    for z in -half_z + 1..half_z {
        let mode = [transverse[0], transverse[1], z];
        add_mode_view(tensor, domain, values, mode, basis.phase(mode, point));
    }
}

fn add_mode_view(
    tensor: &mut Tensor,
    domain: Domain,
    values: [&[Complex64]; 3],
    mode: [isize; 3],
    phase: Complex64,
) {
    let wave = mode.map(|value| std::f64::consts::TAU * value as f64);
    for component in 0..3 {
        add_component(tensor, domain, values, component, mode, phase, wave);
    }
}

fn add_component(
    tensor: &mut Tensor,
    domain: Domain,
    values: [&[Complex64]; 3],
    component: usize,
    mode: [isize; 3],
    phase: Complex64,
    wave: [f64; 3],
) {
    let value = coefficient_view(domain, values, component, mode) * phase;
    tensor.velocity[component] += value.re;
    for a in 0..3 {
        tensor.gradient[component][a] -= wave[a] * value.im;
        for b in 0..3 {
            tensor.hessian[component][a][b] -= wave[a] * wave[b] * value.re;
        }
    }
}

pub(super) fn fields(tensor: &Tensor, reference: ReferenceEvaluation) -> [(Vec<f64>, Vec<f64>); 4] {
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
