//! Forward and inverse transform execution for an admitted plan and workspace.
use super::workspace::finite_real;
use super::{owned, BackendPlan, FftPlan, FftWorkspace};
use crate::spectral::radix::transform;
use crate::{Complex64, SolverError};

impl FftPlan {
    /// Forward R2C coefficients include division by the complete physical sample count.
    /// Nyquist coefficients are represented here; strict-band cropping is a separate operator.
    pub fn forward(
        &self,
        input: &[f64],
        output: &mut [Complex64],
        work: &mut FftWorkspace,
    ) -> Result<(), SolverError> {
        self.validate(input.len(), output.len(), work)?;
        finite_real(input)?;
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for row in 0..nx * ny {
            for k in 0..nz {
                work.input[k] = Complex64::new(input[row * nz + k], 0.0);
            }
            self.transform_axis(
                2,
                false,
                &mut work.input,
                &mut work.output,
                &mut work.scratch,
            );
            work.grid[row * half..(row + 1) * half].copy_from_slice(&work.output[..half]);
        }
        self.transverse(work, false);
        let scale = self.layout.real_len() as f64;
        for (value, transformed) in output.iter_mut().zip(&work.grid) {
            *value = transformed / scale;
        }
        crate::spectral::hermitian::finite(output)
    }

    /// Inverse is the unnormalized Fourier sum. Input must be Hermitian on self-conjugate planes.
    pub fn inverse(
        &self,
        input: &[Complex64],
        output: &mut [f64],
        work: &mut FftWorkspace,
    ) -> Result<(), SolverError> {
        self.validate(output.len(), input.len(), work)?;
        crate::spectral::hermitian::validate(self.layout, input)?;
        work.grid.copy_from_slice(input);
        self.transverse(work, true);
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for row in 0..nx * ny {
            work.input[..half].copy_from_slice(&work.grid[row * half..(row + 1) * half]);
            for k in half..nz {
                work.input[k] = work.input[nz - k].conj();
            }
            self.transform_axis(
                2,
                true,
                &mut work.input,
                &mut work.output,
                &mut work.scratch,
            );
            for k in 0..nz {
                output[row * nz + k] = work.output[k].re;
            }
        }
        finite_real(output)
    }

    fn transform_axis(
        &self,
        axis: usize,
        inverse: bool,
        input: &mut [Complex64],
        output: &mut [Complex64],
        scratch: &mut [Complex64],
    ) {
        let length = self.layout.dimensions()[axis];
        match &self.backend {
            BackendPlan::Owned(roots) => transform(
                &input[..length],
                1,
                &mut output[..length],
                &mut scratch[..length],
                owned::roots(roots, axis),
                inverse,
            ),
            BackendPlan::Avx(axes) => {
                let axes = &axes[0];
                let plan = if inverse {
                    &axes.inverse[axis]
                } else {
                    &axes.forward[axis]
                };
                plan.process_with_scratch(&mut input[..length], scratch);
                output[..length].copy_from_slice(&input[..length]);
            }
        }
    }

    fn transverse(&self, work: &mut FftWorkspace, inverse: bool) {
        for axis in 0..2 {
            self.transverse_axis(work, inverse, axis);
        }
    }

    fn transverse_axis(&self, work: &mut FftWorkspace, inverse: bool, axis: usize) {
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        let (length, rows, stride) = if axis == 0 {
            (nx, ny, ny * half)
        } else {
            (ny, nx, half)
        };
        for row in 0..rows {
            let row_base = if axis == 0 {
                row * half
            } else {
                row * ny * half
            };
            for k in 0..half {
                let base = row_base + k;
                for j in 0..length {
                    work.input[j] = work.grid[base + j * stride];
                }
                self.transform_axis(
                    axis,
                    inverse,
                    &mut work.input,
                    &mut work.output,
                    &mut work.scratch,
                );
                for j in 0..length {
                    work.grid[base + j * stride] = work.output[j];
                }
            }
        }
    }
}
