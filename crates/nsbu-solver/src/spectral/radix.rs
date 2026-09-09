//! Deterministic mixed-radix 2/3 complex FFT with caller-owned scratch.
use crate::Complex64;

/// Decimation in time: recursively transform residue classes, then combine.
/// Supported lengths factor entirely into 2 and 3. No heap allocation occurs.
pub(super) fn transform(
    input: &[Complex64],
    stride: usize,
    output: &mut [Complex64],
    scratch: &mut [Complex64],
    roots: &[Complex64],
    inverse: bool,
) {
    let n = output.len();
    if n == 1 {
        output[0] = input[0];
        return;
    }
    let radix = if n.is_multiple_of(2) { 2 } else { 3 };
    let width = n / radix;
    for j in 0..radix {
        transform(
            &input[j * stride..],
            stride * radix,
            &mut output[j * width..(j + 1) * width],
            &mut scratch[..width],
            roots,
            inverse,
        );
    }
    for (k, value) in scratch[..n].iter_mut().enumerate() {
        *value = Complex64::new(0.0, 0.0);
        for j in 0..radix {
            let root = roots[(j * k * (roots.len() / n)) % roots.len()];
            let twiddle = if inverse { root.conj() } else { root };
            *value += output[j * width + k % width] * twiddle;
        }
    }
    output.copy_from_slice(&scratch[..n]);
}
