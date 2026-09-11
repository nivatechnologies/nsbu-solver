//! Independent signed-mode enumeration and half-spectrum reconstruction.
use nsbu_solver::{domain::SpectralState, Complex64};

pub(super) struct Mode {
    pub(super) wave: [f64; 3],
    pub(super) left: [Complex64; 3],
    pub(super) right: [Complex64; 3],
}

#[derive(Default)]
pub(super) struct Tensor {
    pub(super) velocity: [f64; 3],
    pub(super) gradient: [[f64; 3]; 3],
    pub(super) hessian: [[[f64; 3]; 3]; 3],
}

pub(super) fn coefficient(state: &SpectralState, mode: [isize; 3], axis: usize) -> Complex64 {
    let layout = state.plan().domain().layout();
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
    let value = state.component(axis).unwrap()[index];
    if conjugate {
        value.conj()
    } else {
        value
    }
}

pub(super) fn modes(left: &SpectralState, right: &SpectralState) -> Vec<Mode> {
    let [nx, ny, nz] = right
        .plan()
        .domain()
        .layout()
        .dimensions()
        .map(|n| n as isize / 2);
    let mut result = Vec::new();
    for x in -nx + 1..nx {
        for y in -ny + 1..ny {
            for z in -nz + 1..nz {
                let mode = [x, y, z];
                result.push(Mode {
                    wave: mode.map(|k| std::f64::consts::TAU * k as f64),
                    left: std::array::from_fn(|axis| coefficient(left, mode, axis)),
                    right: std::array::from_fn(|axis| coefficient(right, mode, axis)),
                });
            }
        }
    }
    result
}
