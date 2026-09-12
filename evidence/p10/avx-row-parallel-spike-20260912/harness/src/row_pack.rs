#![allow(clippy::needless_range_loop)]
use crate::transform::{AxisKind, Lane};
use rustfft::num_complex::Complex64;

pub(crate) fn axis_rows(n: usize, half: usize, kind: AxisKind) -> usize {
    match kind {
        AxisKind::RealZ | AxisKind::InverseZ => n * n,
        AxisKind::ForwardX | AxisKind::ForwardY | AxisKind::InverseX | AxisKind::InverseY => {
            n * half
        }
    }
}

pub(crate) fn pack_rows(
    lane: &Lane,
    kind: AxisKind,
    start: usize,
    rows: usize,
    buffer: &mut [Complex64],
) {
    let n = lane.n;
    for local in 0..rows {
        let global = start + local;
        let target = &mut buffer[local * n..(local + 1) * n];
        match kind {
            AxisKind::RealZ => {
                for j in 0..n {
                    target[j] = Complex64::new(lane.physical[global * n + j], 0.0);
                }
            }
            AxisKind::InverseZ => {
                target[..lane.half]
                    .copy_from_slice(&lane.grid[global * lane.half..(global + 1) * lane.half]);
                for j in lane.half..n {
                    target[j] = target[n - j].conj();
                }
            }
            AxisKind::ForwardX | AxisKind::InverseX => {
                let y = global / lane.half;
                let k = global % lane.half;
                let base = y * lane.half + k;
                let stride = n * lane.half;
                for j in 0..n {
                    target[j] = lane.grid[base + j * stride];
                }
            }
            AxisKind::ForwardY | AxisKind::InverseY => {
                let x = global / lane.half;
                let k = global % lane.half;
                let base = x * n * lane.half + k;
                for j in 0..n {
                    target[j] = lane.grid[base + j * lane.half];
                }
            }
        }
    }
}

pub(crate) fn scatter_rows(
    lane: &mut Lane,
    kind: AxisKind,
    start: usize,
    rows: usize,
    buffer: &[Complex64],
) {
    let n = lane.n;
    for local in 0..rows {
        let global = start + local;
        let source = &buffer[local * n..(local + 1) * n];
        match kind {
            AxisKind::RealZ => lane.grid[global * lane.half..(global + 1) * lane.half]
                .copy_from_slice(&source[..lane.half]),
            AxisKind::InverseZ => {
                for j in 0..n {
                    lane.physical[global * n + j] = source[j].re;
                }
            }
            AxisKind::ForwardX | AxisKind::InverseX => {
                let y = global / lane.half;
                let k = global % lane.half;
                let base = y * lane.half + k;
                let stride = n * lane.half;
                for j in 0..n {
                    lane.grid[base + j * stride] = source[j];
                }
            }
            AxisKind::ForwardY | AxisKind::InverseY => {
                let x = global / lane.half;
                let k = global % lane.half;
                let base = x * n * lane.half + k;
                for j in 0..n {
                    lane.grid[base + j * lane.half] = source[j];
                }
            }
        }
    }
}
