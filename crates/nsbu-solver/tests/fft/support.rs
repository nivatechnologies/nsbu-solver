use nsbu_solver::domain::Layout;
use nsbu_solver::Complex64;

#[cfg(target_arch = "x86_64")]
pub(super) fn has_required_avx() -> bool {
    std::is_x86_feature_detected!("avx")
        && std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("fma")
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn has_required_avx() -> bool {
    false
}

pub(super) fn direct(values: &[f64], dimensions: [usize; 3], mode: [usize; 3]) -> Complex64 {
    let [nx, ny, nz] = dimensions;
    let mut sum = Complex64::new(0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let phase = -std::f64::consts::TAU
                    * (i as f64 * mode[0] as f64 / nx as f64
                        + j as f64 * mode[1] as f64 / ny as f64
                        + k as f64 * mode[2] as f64 / nz as f64);
                sum += values[(i * ny + j) * nz + k] * Complex64::new(phase.cos(), phase.sin());
            }
        }
    }
    sum / values.len() as f64
}

pub(super) fn direct_inverse(coefficients: &[Complex64], layout: Layout, point: [usize; 3]) -> f64 {
    let [nx, ny, nz] = layout.dimensions();
    let mut sum = Complex64::new(0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..=nz / 2 {
                let phase = std::f64::consts::TAU
                    * (i as f64 * point[0] as f64 / nx as f64
                        + j as f64 * point[1] as f64 / ny as f64
                        + k as f64 * point[2] as f64 / nz as f64);
                let exponential = Complex64::new(phase.cos(), phase.sin());
                let value = coefficients[layout.index([i, j, k]).unwrap()];
                sum += value * exponential;
                if k != 0 && k != nz / 2 {
                    sum += value.conj() * exponential.conj();
                }
            }
        }
    }
    sum.re
}

pub(super) fn hermitian_fixture(layout: Layout) -> Vec<Complex64> {
    let [nx, ny, nz] = layout.dimensions();
    let mut values = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for k in 1..nz / 2 {
        for i in 0..nx {
            for j in 0..ny {
                values[layout.index([i, j, k]).unwrap()] = Complex64::new(
                    (11 * i + 7 * j + 5 * k) as f64 / 97.0,
                    (3 * i + 13 * j + 2 * k) as f64 / 89.0 - 0.5,
                );
            }
        }
    }
    for k in [0, nz / 2] {
        for i in 0..nx {
            for j in 0..ny {
                let partner = [(nx - i) % nx, (ny - j) % ny, k];
                let position = [i, j, k];
                if layout.index(position).unwrap() > layout.index(partner).unwrap() {
                    continue;
                }
                let value = Complex64::new(
                    (17 * i + 5 * j + 3 * k) as f64 / 101.0,
                    (7 * i + 11 * j + k) as f64 / 103.0 - 0.25,
                );
                let owned = if position == partner {
                    Complex64::new(value.re, 0.0)
                } else {
                    value
                };
                values[layout.index(position).unwrap()] = owned;
                values[layout.index(partner).unwrap()] = owned.conj();
            }
        }
    }
    values
}

pub(super) fn same_bits(left: &[Complex64], right: &[Complex64]) -> bool {
    left.iter()
        .zip(right)
        .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}
