//! Independent Fourier oracle for nonlinear pressure differences.
use nsbu_solver::{
    domain::{Layout, SpectralState},
    Complex64,
};
#[path = "v2_pressure_oracle/convolution.rs"]
mod convolution;
#[path = "v2_pressure_oracle/dft.rs"]
mod dft;
#[path = "v2_pressure_oracle/modes.rs"]
mod modes;
#[derive(Debug, Clone, Copy, Default)]
/// RMS and peak magnitude sampled on the physical grid.
pub struct Norms {
    /// Root mean square magnitude.
    pub rms: f64,
    /// Maximum magnitude.
    pub peak: f64,
}
pub(crate) fn signed(i: usize, n: usize) -> isize {
    if i > n / 2 {
        i as isize - n as isize
    } else {
        i as isize
    }
}
/// Compare nonlinear pressure and its complete three-component gradient.
pub fn pair_norms(left: &SpectralState, right: &SpectralState, samples: Layout) -> [Norms; 2] {
    let (modes, d) = modes::modes(left, right);
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
                        pressure[q] * Complex64::new(0.0, std::f64::consts::TAU * k[a] as f64);
                }
            }
        }
    }
    let pv = dft::inverse(&pressure, d, samples);
    let gv = gradient.map(|v| dft::inverse(&v, d, samples));
    let mut sum = [0.0_f64; 2];
    let mut peak = [0.0_f64; 2];
    for q in 0..samples.real_len() {
        let p = pv[q].re.powi(2);
        let g: f64 = gv.iter().map(|v| v[q].re.powi(2)).sum();
        for (i, v) in [p, g].into_iter().enumerate() {
            sum[i] += v;
            peak[i] = peak[i].max(v);
        }
    }
    std::array::from_fn(|i| Norms {
        rms: (sum[i] / samples.real_len() as f64).sqrt(),
        peak: peak[i].sqrt(),
    })
}
#[test]
fn taylor_green_pressure_and_gradient_are_recovered() {
    let n = 8;
    let d = [n; 3];
    let mut modes = Vec::new();
    for &x in &[-1isize, 1] {
        for &y in &[-1isize, 1] {
            let mut u = [Complex64::new(0.0, 0.0); 3];
            u[0] = Complex64::new(0.0, if x == 1 { -0.25 } else { 0.25 });
            u[1] = Complex64::new(0.0, if y == 1 { 0.25 } else { -0.25 });
            modes.push(modes::Mode {
                k: [x, y, 0],
                left: [Complex64::new(0.0, 0.0); 3],
                right: u,
                delta: u,
            });
        }
    }
    let pressure = convolution::pressure(&modes, d);
    let sample = Layout::new([24; 3]).unwrap();
    let values = dft::inverse(&pressure, d, sample);
    let rms = (values.iter().map(|v| v.re * v.re).sum::<f64>() / values.len() as f64).sqrt();
    assert!((rms - 0.25).abs() < 1e-12);
    let mut g = 0.0;
    for axis in 0..3 {
        let mut derivative = pressure.clone();
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let q = (x * n + y) * n + z;
                    derivative[q] *= Complex64::new(
                        0.0,
                        std::f64::consts::TAU * signed([x, y, z][axis], n) as f64,
                    );
                }
            }
        }
        g += dft::inverse(&derivative, d, sample)
            .iter()
            .map(|v| v.re * v.re)
            .sum::<f64>()
            / values.len() as f64;
    }
    assert!((g.sqrt() - std::f64::consts::PI).abs() < 1e-12);
}

#[test]
fn rectangular_convolution_keeps_negative_modes_on_each_axis() {
    let d = [8, 24, 8];
    let u = [
        Complex64::new(0.0, 0.0),
        Complex64::new(1.0, 0.0),
        Complex64::new(0.0, 0.0),
    ];
    let modes = [-5, 5].map(|y| modes::Mode {
        k: [0, y, 0],
        left: [Complex64::new(0.0, 0.0); 3],
        right: u,
        delta: u,
    });
    let pressure = convolution::pressure(&modes, d);
    // The two self-products have k_y=+/-10; -k_y^2 / |k|^2 = -1.
    // Their mixed zero mode is removed by the fixed mean-zero gauge.
    for (index, value) in pressure.into_iter().enumerate() {
        let expected = if index == 10 * 8 || index == 14 * 8 {
            -1.0
        } else {
            0.0
        };
        assert!((value - Complex64::new(expected, 0.0)).l1_norm() < 1e-14);
    }
}
