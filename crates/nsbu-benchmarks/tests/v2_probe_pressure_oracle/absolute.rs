//! Absolute forced-pressure control using independent mode sums and inverse DFT.
use super::{convolution, dft, modes, signed};
use nsbu_solver::{
    domain::{Domain, Layout},
    Complex64,
};

/// Independently sampled scalar pressure and its three ordered derivatives.
pub struct AbsolutePressure {
    pub scalar: Vec<f64>,
    pub gradient: [Vec<f64>; 3],
}
/// Assemble absolute pressure from velocity and validated force coefficients.
/// The wave-number scale here is for the unit-cube exact-v2 test fixture.
pub fn absolute(
    domain: Domain,
    values: [&[Complex64]; 3],
    force_domain: Domain,
    force: [&[Complex64]; 3],
    samples: Layout,
) -> AbsolutePressure {
    let d = force_domain.layout().dimensions();
    let modes = modes::single(domain, values, d);
    let mut pressure = convolution::pressure(&modes, d);
    for x in 0..d[0] {
        for y in 0..d[1] {
            for z in 0..d[2] {
                let q = (x * d[1] + y) * d[2] + z;
                let k = [signed(x, d[0]), signed(y, d[1]), signed(z, d[2])];
                let wave = k.map(|v| std::f64::consts::TAU * v as f64);
                let den = wave.into_iter().map(|v| v * v).sum::<f64>();
                if den != 0.0 {
                    let div = (0..3)
                        .map(|a| {
                            Complex64::new(0.0, wave[a]) * modes::coeff(force_domain, force, a, k)
                        })
                        .sum::<Complex64>();
                    pressure[q] -= div / den;
                }
            }
        }
    }
    let mut gradient = std::array::from_fn(|_| pressure.clone());
    for x in 0..d[0] {
        for y in 0..d[1] {
            for z in 0..d[2] {
                let q = (x * d[1] + y) * d[2] + z;
                let k = [signed(x, d[0]), signed(y, d[1]), signed(z, d[2])];
                for a in 0..3 {
                    gradient[a][q] *= Complex64::new(0.0, std::f64::consts::TAU * k[a] as f64);
                }
            }
        }
    }
    AbsolutePressure {
        scalar: dft::inverse(&pressure, d, samples)
            .into_iter()
            .map(|v| v.re)
            .collect(),
        gradient: gradient.map(|v| {
            dft::inverse(&v, d, samples)
                .into_iter()
                .map(|z| z.re)
                .collect()
        }),
    }
}
