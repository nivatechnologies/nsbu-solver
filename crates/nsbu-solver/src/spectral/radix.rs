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
    if radix == 2 {
        combine_two(output, &mut scratch[..n], roots, inverse);
    } else {
        combine_three(output, &mut scratch[..n], roots, inverse);
    }
    output.copy_from_slice(&scratch[..n]);
}

fn combine_two(input: &[Complex64], output: &mut [Complex64], roots: &[Complex64], inverse: bool) {
    let width = output.len() / 2;
    let step = roots.len() / output.len();
    let zeroth = if inverse { roots[0].conj() } else { roots[0] };
    for quotient in 0..2 {
        for remainder in 0..width {
            let k = quotient * width + remainder;
            output[k] = Complex64::new(0.0, 0.0);
            output[k] += input[remainder] * zeroth;
            let root = roots[k * step];
            let twiddle = if inverse { root.conj() } else { root };
            output[k] += input[width + remainder] * twiddle;
        }
    }
}

fn combine_three(
    input: &[Complex64],
    output: &mut [Complex64],
    roots: &[Complex64],
    inverse: bool,
) {
    let width = output.len() / 3;
    let step = roots.len() / output.len();
    let zeroth = if inverse { roots[0].conj() } else { roots[0] };
    for quotient in 0..3 {
        for remainder in 0..width {
            let k = quotient * width + remainder;
            output[k] = Complex64::new(0.0, 0.0);
            output[k] += input[remainder] * zeroth;
            let first = roots[k * step];
            let first = if inverse { first.conj() } else { first };
            output[k] += input[width + remainder] * first;
            let second_index = 2 * k * step;
            let second_index = if second_index >= roots.len() {
                second_index - roots.len()
            } else {
                second_index
            };
            let second = roots[second_index];
            let second = if inverse { second.conj() } else { second };
            output[k] += input[2 * width + remainder] * second;
        }
    }
}
