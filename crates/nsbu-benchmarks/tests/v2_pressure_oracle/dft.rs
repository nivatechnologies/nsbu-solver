use nsbu_solver::{domain::Layout, Complex64};
fn phase(n: usize, k: isize, j: usize) -> Complex64 {
    let (s, c) = (std::f64::consts::TAU * k as f64 * j as f64 / n as f64).sin_cos();
    Complex64::new(c, s)
}
fn axis_sum(
    input: &[Complex64],
    output: &mut [Complex64],
    length: usize,
    samples: usize,
    stride: usize,
    phases: &[Vec<Complex64>],
) {
    for (base, value) in output.iter_mut().enumerate() {
        let out_axis = (base / stride) % samples;
        let outer = base / (samples * stride);
        let mut sum = Complex64::new(0.0, 0.0);
        for k in -(length as isize / 2) + 1..length as isize / 2 {
            let source =
                (outer * length + k.rem_euclid(length as isize) as usize) * stride + base % stride;
            sum += input[source] * phases[out_axis][(k + length as isize / 2 - 1) as usize];
        }
        *value = sum;
    }
}
pub fn inverse(input: &[Complex64], d: [usize; 3], samples: Layout) -> Vec<Complex64> {
    let [sx, sy, sz] = samples.dimensions();
    let [nx, ny, nz] = d;
    let px: Vec<Vec<_>> = (0..sx)
        .map(|j| {
            (-(nx as isize / 2) + 1..nx as isize / 2)
                .map(|k| phase(sx, k, j))
                .collect()
        })
        .collect();
    let py: Vec<Vec<_>> = (0..sy)
        .map(|j| {
            (-(ny as isize / 2) + 1..ny as isize / 2)
                .map(|k| phase(sy, k, j))
                .collect()
        })
        .collect();
    let pz: Vec<Vec<_>> = (0..sz)
        .map(|j| {
            (-(nz as isize / 2) + 1..nz as isize / 2)
                .map(|k| phase(sz, k, j))
                .collect()
        })
        .collect();
    let mut a = vec![Complex64::new(0.0, 0.0); sx * ny * nz];
    axis_sum(input, &mut a, nx, sx, ny * nz, &px);
    let mut b = vec![Complex64::new(0.0, 0.0); sx * sy * nz];
    axis_sum(&a, &mut b, ny, sy, nz, &py);
    let mut out = vec![Complex64::new(0.0, 0.0); sx * sy * sz];
    axis_sum(&b, &mut out, nz, sz, 1, &pz);
    out
}
