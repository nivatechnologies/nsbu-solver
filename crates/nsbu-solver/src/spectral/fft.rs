//! Bounded small CPU FFT backend with explicit normalization and owned scratch.
mod avx;
use super::radix::transform;
use crate::storage::filled;
use crate::{domain::Layout, Complex64, SolverError};
pub use avx::FftCatalog;

/// Immutable arithmetic/backend identity for every scalar transform owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FftBackend {
    /// Project-owned mixed-radix binary64 arithmetic; the default compatibility profile.
    OwnedRadix,
    /// RustFFT 6.4.1's explicit x86_64 AVX/FMA planner on a closed length set.
    RustFft6_4_1AvxFma,
}

impl FftBackend {
    /// Refuse an unavailable execution backend without planning or allocating storage.
    pub fn ensure_available(self) -> Result<(), SolverError> {
        match self {
            Self::OwnedRadix => Ok(()),
            Self::RustFft6_4_1AvxFma => avx::ensure_available(),
        }
    }
}

enum BackendPlan {
    Owned(Vec<Complex64>),
    Avx(Box<[avx::Axes]>),
}

/// Immutable plan for one explicitly selected scalar-transform backend.
pub struct FftPlan {
    layout: Layout,
    backend: BackendPlan,
    _layout_compatibility: [u8; 48],
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
        Self::reservation_with_backend(layout, FftBackend::OwnedRadix)
    }

    /// Complete fixed reservation for the selected immutable arithmetic backend.
    pub fn reservation_with_backend(
        layout: Layout,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        match backend {
            FftBackend::OwnedRadix => owned_reservation(layout),
            FftBackend::RustFft6_4_1AvxFma => avx::reservation(layout),
        }
    }

    /// Preflight and allocate the default project-owned radix backend.
    pub fn new(layout: Layout, cap: usize) -> Result<(Self, FftWorkspace), SolverError> {
        Self::new_with_backend(layout, FftBackend::OwnedRadix, cap)
    }

    /// Preflight and allocate one explicitly identified backend and its fixed scratch.
    pub fn new_with_backend(
        layout: Layout,
        backend: FftBackend,
        cap: usize,
    ) -> Result<(Self, FftWorkspace), SolverError> {
        let reservation = Self::reservation_with_backend(layout, backend)?;
        if reservation > cap {
            return Err(SolverError::ResourceLimit);
        }
        match backend {
            FftBackend::OwnedRadix => owned_new(layout),
            FftBackend::RustFft6_4_1AvxFma => avx::new(layout),
        }
    }

    /// Workspace reservation when immutable AVX plans are owned by an admitted catalog.
    pub fn reservation_from_catalog(
        layout: Layout,
        catalog: &FftCatalog,
    ) -> Result<usize, SolverError> {
        Self::reservation_with_shared_backend(layout, catalog.backend())
    }

    /// Workspace-only reservation when the enclosing execution owns immutable plans.
    pub fn reservation_with_shared_backend(
        layout: Layout,
        backend: FftBackend,
    ) -> Result<usize, SolverError> {
        match backend {
            FftBackend::OwnedRadix => owned_reservation(layout),
            FftBackend::RustFft6_4_1AvxFma => avx::workspace_reservation(layout),
        }
    }

    /// Allocate one scalar workspace while reusing an execution-owned immutable plan catalog.
    pub fn new_from_catalog(
        layout: Layout,
        catalog: &FftCatalog,
        cap: usize,
    ) -> Result<(Self, FftWorkspace), SolverError> {
        let reservation = Self::reservation_from_catalog(layout, catalog)?;
        if reservation > cap {
            return Err(SolverError::ResourceLimit);
        }
        match catalog.backend() {
            FftBackend::OwnedRadix => owned_new(layout),
            FftBackend::RustFft6_4_1AvxFma => avx::from_catalog(layout, catalog),
        }
    }

    /// Immutable backend identity bound to this plan.
    pub fn backend(&self) -> FftBackend {
        match self.backend {
            BackendPlan::Owned(_) => FftBackend::OwnedRadix,
            BackendPlan::Avx(_) => FftBackend::RustFft6_4_1AvxFma,
        }
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
                owned_roots(self.layout.dimensions(), roots, axis),
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

    fn validate(&self, real: usize, half: usize, work: &FftWorkspace) -> Result<(), SolverError> {
        let maximum = self
            .layout
            .dimensions()
            .into_iter()
            .max()
            .ok_or(SolverError::InvalidDomain)?;
        let required_scratch = match &self.backend {
            BackendPlan::Owned(_) => maximum,
            BackendPlan::Avx(axes) => axes[0]
                .forward
                .iter()
                .chain(&axes[0].inverse)
                .map(|plan| plan.get_inplace_scratch_len())
                .max()
                .ok_or(SolverError::InvalidDomain)?,
        };
        if real != self.layout.real_len()
            || half != self.layout.half_len()
            || work.layout != self.layout
            || work.grid.len() != self.layout.half_len()
            || work.input.len() != maximum
            || work.output.len() != maximum
            || work.scratch.len() < required_scratch
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

impl std::fmt::Debug for FftPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FftPlan")
            .field("layout", &self.layout)
            .field("backend", &self.backend())
            .finish()
    }
}

fn owned_reservation(layout: Layout) -> Result<usize, SolverError> {
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
            value.checked_add(std::mem::size_of::<FftPlan>() + std::mem::size_of::<FftWorkspace>())
        })
        .ok_or(SolverError::SizeOverflow)
}

fn owned_new(layout: Layout) -> Result<(FftPlan, FftWorkspace), SolverError> {
    let dimensions = layout.dimensions();
    let mut roots = filled(dimensions.iter().sum(), Complex64::new(0.0, 0.0))?;
    let mut offset = 0;
    for length in dimensions {
        roots[offset..offset + length].copy_from_slice(&roots_for_length(length)?);
        offset += length;
    }
    let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
    let workspace = FftWorkspace {
        layout,
        grid: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        input: filled(maximum, Complex64::new(0.0, 0.0))?,
        output: filled(maximum, Complex64::new(0.0, 0.0))?,
        scratch: filled(maximum, Complex64::new(0.0, 0.0))?,
    };
    Ok((
        FftPlan {
            layout,
            backend: BackendPlan::Owned(roots),
            _layout_compatibility: [0; 48],
        },
        workspace,
    ))
}

fn roots_for_length(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = filled(n, Complex64::new(0.0, 0.0))?;
    for (k, value) in values.iter_mut().enumerate() {
        let angle = -std::f64::consts::TAU * k as f64 / n as f64;
        *value = Complex64::new(angle.cos(), angle.sin());
    }
    Ok(values)
}

fn owned_roots(dimensions: [usize; 3], roots: &[Complex64], axis: usize) -> &[Complex64] {
    let start = dimensions[..axis].iter().sum::<usize>();
    &roots[start..start + dimensions[axis]]
}

fn finite_real(values: &[f64]) -> Result<(), SolverError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}
