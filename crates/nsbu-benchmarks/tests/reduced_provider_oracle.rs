//! Independent physical sample traversal and direct DFT for the optional reduced provider.
use nsbu_benchmarks::{reduced_force, time::BenchmarkTime};
use nsbu_solver::{domain::Layout, Complex64};

/// Force samples in independently enumerated physical storage, plus actual plane/root work.
pub fn samples(layout: Layout, time: BenchmarkTime) -> (Vec<[f64; 3]>, usize, usize) {
    let [nx, ny, nz] = layout.dimensions();
    let mut values = Vec::with_capacity(layout.real_len());
    let mut planes = 0;
    let mut uncached = 0;
    for x in 0..nx {
        for y in 0..ny {
            for z in 0..nz {
                let point = [
                    x as f64 / nx as f64,
                    y as f64 / ny as f64,
                    z as f64 / nz as f64,
                ];
                let sample = reduced_force::evaluate(point, time).unwrap();
                let count = sample.root.map_or(0, |report| report.iterations);
                uncached += count;
                if x == 0 && y == 0 {
                    planes += count;
                }
                values.push(sample.force);
            }
        }
    }
    (values, planes, uncached)
}

/// Normalized unprojected coefficients from a direct complex sum; no production FFT/indexing.
pub fn dft(retained: Layout, sampled: Layout, values: &[[f64; 3]]) -> [Vec<Complex64>; 3] {
    let [nx, ny, nz] = retained.dimensions();
    let mut result: [Vec<Complex64>; 3] = std::array::from_fn(|_| Vec::new());
    for x in 0..nx {
        for y in 0..ny {
            for z in 0..=nz / 2 {
                let mode = [signed(x, nx), signed(y, ny), z as isize];
                let forbidden = x == nx / 2 || y == ny / 2 || z == nz / 2;
                let coefficient = if forbidden {
                    [Complex64::new(0.0, 0.0); 3]
                } else {
                    sum_mode(mode, sampled.dimensions(), values)
                };
                for (axis, value) in coefficient.into_iter().enumerate() {
                    result[axis].push(value);
                }
            }
        }
    }
    result
}

fn signed(index: usize, size: usize) -> isize {
    if index < size / 2 {
        index as isize
    } else {
        index as isize - size as isize
    }
}

fn sum_mode(mode: [isize; 3], [nx, ny, nz]: [usize; 3], values: &[[f64; 3]]) -> [Complex64; 3] {
    let mut result = [Complex64::new(0.0, 0.0); 3];
    for (linear, force) in values.iter().enumerate() {
        let point = [
            (linear / (ny * nz)) as f64 / nx as f64,
            ((linear / nz) % ny) as f64 / ny as f64,
            (linear % nz) as f64 / nz as f64,
        ];
        let phase = -std::f64::consts::TAU
            * mode
                .into_iter()
                .zip(point)
                .map(|(k, x)| k as f64 * x)
                .sum::<f64>();
        let (sin, cos) = phase.sin_cos();
        for axis in 0..3 {
            result[axis] += Complex64::new(cos, sin) * force[axis];
        }
    }
    result.map(|z| z / values.len() as f64)
}

/// Maximum scaled complete-spectrum discrepancy, with shape checks before comparison.
pub fn compare(
    actual: &[Vec<Complex64>; 3],
    expected: &[Vec<Complex64>; 3],
    tolerance: f64,
) -> f64 {
    let mut maximum = 0.0_f64;
    for (a, b) in actual.iter().zip(expected) {
        assert_eq!(a.len(), b.len());
        for (index, (a, b)) in a.iter().zip(b).enumerate() {
            let error = (*a - *b).l1_norm() / (1.0 + b.l1_norm());
            assert!(
                error < tolerance,
                "coefficient {index}: {a:?} != {b:?}, scaled {error:e}"
            );
            maximum = maximum.max(error);
        }
    }
    maximum
}
