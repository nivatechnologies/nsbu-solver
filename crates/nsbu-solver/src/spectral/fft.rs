//! Bounded small CPU FFT backend with explicit normalization and owned scratch.
use super::radix::transform;
use crate::storage::filled;
use crate::{domain::Layout, Complex64, SolverError};

/// Immutable radix-2/3 roots. Grid lengths are limited to 1024 per axis.
#[derive(Debug)]
pub struct FftPlan {
    layout: Layout,
    roots: [Vec<Complex64>; 3],
}

/// Mutable storage belonging to one scalar transform stream, never shared.
#[derive(Debug)]
pub struct FftWorkspace {
    layout: Layout,
    grid: Vec<Complex64>,
    input: Vec<Complex64>,
    output: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl FftPlan {
    /// Exact element-storage reservation for roots and one workspace, plus object headers.
    /// The caller separately reserves allocator overhead and input/output buffers.
    pub fn reservation(layout: Layout) -> Result<usize, SolverError> {
        let dimensions = layout.dimensions();
        for n in dimensions {
            let mut factor = n;
            for radix in [2, 3] {
                for _ in 0..usize::BITS {
                    if !factor.is_multiple_of(radix) {
                        break;
                    }
                    factor /= radix;
                }
            }
            if factor != 1 || n > 1024 {
                return Err(SolverError::InvalidDomain);
            }
        }
        let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
        let elements = layout
            .half_len()
            .checked_add(3 * maximum + dimensions.iter().sum::<usize>())
            .ok_or(SolverError::SizeOverflow)?;
        elements
            .checked_mul(16)
            .and_then(|value| {
                value.checked_add(std::mem::size_of::<Self>() + std::mem::size_of::<FftWorkspace>())
            })
            .ok_or(SolverError::SizeOverflow)
    }

    /// Preflight and allocate fixed roots/scratch. No hidden planner or native allocation.
    pub fn new(layout: Layout, cap: usize) -> Result<(Self, FftWorkspace), SolverError> {
        if Self::reservation(layout)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let dimensions = layout.dimensions();
        let roots = [
            roots(dimensions[0])?,
            roots(dimensions[1])?,
            roots(dimensions[2])?,
        ];
        let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
        let workspace = FftWorkspace {
            layout,
            grid: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
            input: filled(maximum, Complex64::new(0.0, 0.0))?,
            output: filled(maximum, Complex64::new(0.0, 0.0))?,
            scratch: filled(maximum, Complex64::new(0.0, 0.0))?,
        };
        Ok((Self { layout, roots }, workspace))
    }

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
            transform(
                &work.input[..nz],
                1,
                &mut work.output[..nz],
                &mut work.scratch[..nz],
                &self.roots[2],
                false,
            );
            work.grid[row * half..(row + 1) * half].copy_from_slice(&work.output[..half]);
        }
        self.transverse(work, false);
        let scale = self.layout.real_len() as f64;
        for (value, transformed) in output.iter_mut().zip(&work.grid) {
            *value = transformed / scale;
        }
        super::hermitian::finite(output)
    }

    /// Inverse is the unnormalized Fourier sum. Input must be Hermitian on self-conjugate planes.
    pub fn inverse(
        &self,
        input: &[Complex64],
        output: &mut [f64],
        work: &mut FftWorkspace,
    ) -> Result<(), SolverError> {
        self.validate(output.len(), input.len(), work)?;
        super::hermitian::validate(self.layout, input)?;
        work.grid.copy_from_slice(input);
        self.transverse(work, true);
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for row in 0..nx * ny {
            work.input[..half].copy_from_slice(&work.grid[row * half..(row + 1) * half]);
            for k in half..nz {
                work.input[k] = work.input[nz - k].conj();
            }
            transform(
                &work.input[..nz],
                1,
                &mut work.output[..nz],
                &mut work.scratch[..nz],
                &self.roots[2],
                true,
            );
            for k in 0..nz {
                output[row * nz + k] = work.output[k].re;
            }
        }
        finite_real(output)
    }

    fn validate(&self, real: usize, half: usize, work: &FftWorkspace) -> Result<(), SolverError> {
        if real != self.layout.real_len()
            || half != self.layout.half_len()
            || work.layout != self.layout
        {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
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
                transform(
                    &work.input[..length],
                    1,
                    &mut work.output[..length],
                    &mut work.scratch[..length],
                    &self.roots[axis],
                    inverse,
                );
                for j in 0..length {
                    work.grid[base + j * stride] = work.output[j];
                }
            }
        }
    }
}

fn roots(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = filled(n, Complex64::new(0.0, 0.0))?;
    for (k, value) in values.iter_mut().enumerate() {
        let angle = -std::f64::consts::TAU * k as f64 / n as f64;
        *value = Complex64::new(angle.cos(), angle.sin());
    }
    Ok(values)
}

fn finite_real(values: &[f64]) -> Result<(), SolverError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}
