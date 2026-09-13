//! Backend-local tiled transverse traversal for the AVX FFT profile.
use super::*;
use crate::spectral::fft::TRANSVERSE_TILE_LANES;

impl FftPlan {
    pub(super) fn transverse_axis_tiled(
        &self,
        work: &mut FftWorkspace,
        inverse: bool,
        axis: usize,
    ) {
        let [nx, ny, nz] = self.layout.dimensions();
        let maximum = nx.max(ny).max(nz);
        let half = nz / 2 + 1;
        let (length, rows, stride) = if axis == 0 {
            (nx, ny, ny * half)
        } else {
            (ny, nx, half)
        };
        let prefix = AVX_SCRATCH_LANES * maximum;
        for row in 0..rows {
            let row_base = if axis == 0 {
                row * half
            } else {
                row * ny * half
            };
            for k_start in (0..half).step_by(TRANSVERSE_TILE_LANES) {
                let width = (half - k_start).min(TRANSVERSE_TILE_LANES);
                {
                    let tile = &mut work.scratch[prefix..];
                    gather_tile(&work.grid, tile, row_base + k_start, width, length, stride);
                }
                self.transform_transverse_tile(work, inverse, axis, width, length, prefix);
                {
                    let tile = &work.scratch[prefix..];
                    scatter_tile(
                        &mut work.grid,
                        tile,
                        row_base + k_start,
                        width,
                        length,
                        stride,
                    );
                }
            }
        }
    }

    fn transform_transverse_tile(
        &self,
        work: &mut FftWorkspace,
        inverse: bool,
        axis: usize,
        width: usize,
        length: usize,
        prefix: usize,
    ) {
        for lane in 0..width {
            let range = prefix + lane * length..prefix + (lane + 1) * length;
            work.input[..length].copy_from_slice(&work.scratch[range.clone()]);
            self.transform_axis(
                axis,
                inverse,
                &mut work.input,
                &mut work.output,
                &mut work.scratch[..prefix],
            );
            work.scratch[range].copy_from_slice(&work.output[..length]);
        }
    }
}

fn gather_tile(
    grid: &[Complex64],
    tile: &mut [Complex64],
    base: usize,
    width: usize,
    length: usize,
    stride: usize,
) {
    for j in 0..length {
        let source = base + j * stride;
        for lane in 0..width {
            tile[lane * length + j] = grid[source + lane];
        }
    }
}

fn scatter_tile(
    grid: &mut [Complex64],
    tile: &[Complex64],
    base: usize,
    width: usize,
    length: usize,
    stride: usize,
) {
    for j in 0..length {
        let target = base + j * stride;
        for lane in 0..width {
            grid[target + lane] = tile[lane * length + j];
        }
    }
}
