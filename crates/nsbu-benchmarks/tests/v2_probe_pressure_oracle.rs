//! Independent full-complex pressure-difference oracle for arbitrary spectral views.
use nsbu_solver::{
    domain::{Domain, Layout},
    Complex64,
};
#[path = "v2_pressure_oracle/convolution.rs"]
mod convolution;
#[path = "v2_pressure_oracle/dft.rs"]
mod dft;
mod modes {
    use super::*;
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
    pub fn coeff(
        domain: Domain,
        values: [&[Complex64]; 3],
        axis: usize,
        k: [isize; 3],
    ) -> Complex64 {
        let d = domain.layout().dimensions();
        if k.iter().zip(d).any(|(&v, n)| v.unsigned_abs() >= n / 2) {
            return Complex64::new(0.0, 0.0);
        }
        let conjugate = k[2] < 0;
        let p = std::array::from_fn(|a| {
            (if conjugate { -k[a] } else { k[a] }).rem_euclid(d[a] as isize) as usize
        });
        let v = values[axis][index(p, d)];
        if conjugate {
            v.conj()
        } else {
            v
        }
    }
    pub fn views(
        ld: Domain,
        lv: [&[Complex64]; 3],
        rd: Domain,
        rv: [&[Complex64]; 3],
    ) -> (Vec<Mode>, [usize; 3]) {
        let dl = ld.layout().dimensions();
        let dr = rd.layout().dimensions();
        let d = std::array::from_fn(|a| dl[a].max(dr[a]));
        let mut out = Vec::new();
        for x in -(d[0] as isize / 2) + 1..d[0] as isize / 2 {
            for y in -(d[1] as isize / 2) + 1..d[1] as isize / 2 {
                for z in -(d[2] as isize / 2) + 1..d[2] as isize / 2 {
                    let k = [x, y, z];
                    let left = std::array::from_fn(|a| coeff(ld, lv, a, k));
                    let right = std::array::from_fn(|a| coeff(rd, rv, a, k));
                    out.push(Mode {
                        k,
                        left,
                        right,
                        delta: std::array::from_fn(|a| right[a] - left[a]),
                    });
                }
            }
        }
        (out, d.map(|n| n * 2))
    }

    pub fn single(domain: Domain, values: [&[Complex64]; 3], d: [usize; 3]) -> Vec<Mode> {
        let mut out = Vec::new();
        for x in -(d[0] as isize / 2) + 1..d[0] as isize / 2 {
            for y in -(d[1] as isize / 2) + 1..d[1] as isize / 2 {
                for z in -(d[2] as isize / 2) + 1..d[2] as isize / 2 {
                    let k = [x, y, z];
                    let right = std::array::from_fn(|a| coeff(domain, values, a, k));
                    out.push(Mode {
                        k,
                        left: [Complex64::new(0.0, 0.0); 3],
                        right,
                        delta: right,
                    });
                }
            }
        }
        out
    }
}
#[derive(Clone, Copy)]
pub struct Norms {
    pub rms: f64,
    pub peak: f64,
}
pub fn pair(
    ld: Domain,
    lv: [&[Complex64]; 3],
    rd: Domain,
    rv: [&[Complex64]; 3],
    samples: Layout,
) -> [Norms; 2] {
    let (modes, d) = modes::views(ld, lv, rd, rv);
    let pressure = convolution::pressure(&modes, d);
    let mut gradient: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); pressure.len()]);
    for x in 0..d[0] {
        for y in 0..d[1] {
            for z in 0..d[2] {
                let q = (x * d[1] + y) * d[2] + z;
                let k = [signed(x, d[0]), signed(y, d[1]), signed(z, d[2])];
                for a in 0..3 {
                    gradient[a][q] =
                        pressure[q] * Complex64::new(0.0, std::f64::consts::TAU * k[a] as f64)
                }
            }
        }
    }
    let pv = dft::inverse(&pressure, d, samples);
    let gv = gradient.map(|v| dft::inverse(&v, d, samples));
    let mut sum = [0.0; 2];
    let mut peak: [f64; 2] = [0.0; 2];
    for q in 0..samples.real_len() {
        for (i, v) in [pv[q].re.powi(2), gv.iter().map(|v| v[q].re.powi(2)).sum()]
            .into_iter()
            .enumerate()
        {
            sum[i] += v;
            peak[i] = peak[i].max(v)
        }
    }
    std::array::from_fn(|i| Norms {
        rms: (sum[i] / samples.real_len() as f64).sqrt(),
        peak: peak[i].sqrt(),
    })
}
fn signed(i: usize, n: usize) -> isize {
    if i > n / 2 {
        i as isize - n as isize
    } else {
        i as isize
    }
}

mod absolute;
pub use absolute::absolute;
