//! Ordered quadratic convolution; no production spectral operator is reused.
use super::modes::Mode;
use nsbu_solver::Complex64;

fn pair(l: Mode, r: Mode, d: [usize; 3], out: &mut [Complex64]) {
    let k = [l.k[0] + r.k[0], l.k[1] + r.k[1], l.k[2] + r.k[2]];
    if k == [0; 3] || k.iter().zip(d).any(|(&v, n)| v.unsigned_abs() >= n / 2) {
        return;
    }
    let wave = k.map(|v| std::f64::consts::TAU * v as f64);
    let q: [usize; 3] = std::array::from_fn(|axis| k[axis].rem_euclid(d[axis] as isize) as usize);
    let index = (q[0] * d[1] + q[1]) * d[2] + q[2];
    for i in 0..3 {
        for j in 0..3 {
            out[index] += (l.delta[i] * r.right[j] + l.left[i] * r.delta[j]) * wave[i] * wave[j];
        }
    }
}

pub fn pressure(modes: &[Mode], d: [usize; 3]) -> Vec<Complex64> {
    let mut out = vec![Complex64::new(0.0, 0.0); d[0] * d[1] * d[2]];
    for &l in modes {
        for &r in modes {
            pair(l, r, d, &mut out);
        }
    }
    for x in 0..d[0] {
        for y in 0..d[1] {
            for z in 0..d[2] {
                let q = (x * d[1] + y) * d[2] + z;
                let k = [
                    super::signed(x, d[0]),
                    super::signed(y, d[1]),
                    super::signed(z, d[2]),
                ];
                let den = k
                    .map(|v| (std::f64::consts::TAU * v as f64).powi(2))
                    .into_iter()
                    .sum::<f64>();
                if den != 0.0 {
                    out[q] = -out[q] / den;
                }
            }
        }
    }
    out
}
