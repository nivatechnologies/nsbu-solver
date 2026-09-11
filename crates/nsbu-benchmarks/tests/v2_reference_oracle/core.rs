use nsbu_benchmarks::fields::reference::ReferenceEvaluation;
use nsbu_solver::{domain::SpectralState, Complex64};

#[derive(Default)]
pub(super) struct Tensor {
    pub(super) velocity: [f64; 3],
    pub(super) gradient: [[f64; 3]; 3],
    pub(super) hessian: [[[f64; 3]; 3]; 3],
}

pub(super) fn coefficient(state: &SpectralState, component: usize, mode: [isize; 3]) -> Complex64 {
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

pub(super) fn actual(
    state: &SpectralState,
    point: [usize; 3],
    basis: &super::phase::Basis,
) -> Tensor {
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
