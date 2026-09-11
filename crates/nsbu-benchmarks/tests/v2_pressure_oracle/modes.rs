//! Independent full-complex mode extraction from half-spectrum state words.
use nsbu_solver::Complex64;

#[derive(Clone, Copy)]
pub struct Mode {
    pub k: [isize; 3],
    pub left: [Complex64; 3],
    pub right: [Complex64; 3],
    pub delta: [Complex64; 3],
}

fn index(p: [usize; 3], d: [usize; 3]) -> usize {
    (p[0] * d[1] + p[1]) * (d[2] / 2 + 1) + p[2]
}
fn coeff(state: &nsbu_solver::domain::SpectralState, axis: usize, k: [isize; 3]) -> Complex64 {
    let d = state.plan().domain().layout().dimensions();
    if k.iter().zip(d).any(|(&v, n)| v.unsigned_abs() >= n / 2) {
        return Complex64::new(0.0, 0.0);
    }
    let conjugate = k[2] < 0;
    let p = std::array::from_fn(|a| {
        (if conjugate { -k[a] } else { k[a] }).rem_euclid(d[a] as isize) as usize
    });
    let value = state.component(axis).unwrap()[index(p, d)];
    if conjugate {
        value.conj()
    } else {
        value
    }
}

pub fn modes(
    left: &nsbu_solver::domain::SpectralState,
    right: &nsbu_solver::domain::SpectralState,
) -> (Vec<Mode>, [usize; 3]) {
    let dl = left.plan().domain().layout().dimensions();
    let dr = right.plan().domain().layout().dimensions();
    let d = std::array::from_fn(|a| dl[a].max(dr[a]));
    let mut out = Vec::new();
    for x in -(d[0] as isize / 2) + 1..d[0] as isize / 2 {
        for y in -(d[1] as isize / 2) + 1..d[1] as isize / 2 {
            for z in -(d[2] as isize / 2) + 1..d[2] as isize / 2 {
                let k = [x, y, z];
                let l = std::array::from_fn(|a| coeff(left, a, k));
                let r = std::array::from_fn(|a| coeff(right, a, k));
                out.push(Mode {
                    k,
                    left: l,
                    right: r,
                    delta: std::array::from_fn(|a| r[a] - l[a]),
                });
            }
        }
    }
    (out, d.map(|n| n * 2))
}
