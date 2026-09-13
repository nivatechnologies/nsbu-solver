//! Closed-size RustFFT AVX plan ownership and mutable workspace construction.
use super::{
    BackendPlan, FftBackend, FftPlan, FftWorkspace, AVX_SCRATCH_LANES, TRANSVERSE_TILE_LANES,
};
use crate::storage::filled;
use crate::{domain::Layout, Complex64, SolverError};
use rustfft::{Fft, FftDirection};
use std::sync::Arc;

const LENGTHS: [usize; 14] = [
    6, 96, 128, 144, 192, 256, 288, 384, 512, 576, 768, 1024, 1152, 1536,
];
const PLAN_BYTES_ALLOWANCE: usize = 1024 * 1024;
const PLAN_COUNT_PER_SCALAR: usize = 6;
const ALLOCATION_ALLOWANCE: usize = 64;

pub(super) struct Axes {
    pub(super) forward: [Arc<dyn Fft<f64>>; 3],
    pub(super) inverse: [Arc<dyn Fft<f64>>; 3],
}

struct CatalogEntry {
    length: usize,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
}

/// Execution-owned immutable one-dimensional plan catalog.
pub struct FftCatalog {
    backend: FftBackend,
    avx: Vec<CatalogEntry>,
}

impl FftCatalog {
    /// Accounting allowance: one MiB for each direction and admitted length,
    /// allocator allowance for every catalog slot, and fixed vector/object headers.
    pub fn reservation(backend: FftBackend) -> Result<usize, SolverError> {
        match backend {
            FftBackend::OwnedRadix => Ok(0),
            FftBackend::RustFft6_4_1AvxFma => LENGTHS
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_mul(PLAN_BYTES_ALLOWANCE + ALLOCATION_ALLOWANCE))
                .and_then(|n| n.checked_add(LENGTHS.len() * size_of::<CatalogEntry>()))
                .ok_or(SolverError::SizeOverflow),
        }
    }

    /// Eagerly construct every admitted plan before numerical workspace allocation.
    pub fn new(backend: FftBackend, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(backend)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        match backend {
            FftBackend::OwnedRadix => Ok(Self {
                backend,
                avx: Vec::new(),
            }),
            FftBackend::RustFft6_4_1AvxFma => make_catalog(),
        }
    }

    /// Immutable arithmetic/backend identity of all plans in this catalog.
    pub fn backend(&self) -> FftBackend {
        self.backend
    }

    fn plan(
        &self,
        length: usize,
        direction: FftDirection,
    ) -> Result<Arc<dyn Fft<f64>>, SolverError> {
        let entry = self
            .avx
            .iter()
            .find(|entry| entry.length == length)
            .ok_or(SolverError::InvalidDomain)?;
        Ok(match direction {
            FftDirection::Forward => Arc::clone(&entry.forward),
            FftDirection::Inverse => Arc::clone(&entry.inverse),
        })
    }
}

impl std::fmt::Debug for FftCatalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FftCatalog")
            .field("backend", &self.backend)
            .field("planned_lengths", &self.avx.len())
            .finish()
    }
}

pub(super) fn reservation(layout: Layout) -> Result<usize, SolverError> {
    workspace_reservation(layout)?
        .checked_add(PLAN_COUNT_PER_SCALAR * (PLAN_BYTES_ALLOWANCE + ALLOCATION_ALLOWANCE))
        .ok_or(SolverError::SizeOverflow)
}

pub(super) fn workspace_reservation(layout: Layout) -> Result<usize, SolverError> {
    let dimensions = validate_layout(layout)?;
    let maximum = *dimensions.iter().max().ok_or(SolverError::InvalidDomain)?;
    let workspace_elements = maximum
        .checked_mul(2 + AVX_SCRATCH_LANES + TRANSVERSE_TILE_LANES)
        .and_then(|n| layout.half_len().checked_add(n))
        .ok_or(SolverError::SizeOverflow)?;
    workspace_elements
        .checked_mul(size_of::<Complex64>())
        .and_then(|n| n.checked_add(size_of::<Axes>() + ALLOCATION_ALLOWANCE))
        .and_then(|n| n.checked_add(size_of::<FftPlan>() + size_of::<FftWorkspace>()))
        .ok_or(SolverError::SizeOverflow)
}

#[cfg(target_arch = "x86_64")]
pub(super) fn new(layout: Layout) -> Result<(FftPlan, FftWorkspace), SolverError> {
    require_features()?;
    let dimensions = validate_layout(layout)?;
    let mut planner =
        rustfft::FftPlannerAvx::<f64>::new().map_err(|_| SolverError::InvalidDomain)?;
    let forward = dimensions.map(|n| planner.plan_fft(n, FftDirection::Forward));
    let inverse = dimensions.map(|n| planner.plan_fft(n, FftDirection::Inverse));
    from_parts(layout, forward, inverse)
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn new(_layout: Layout) -> Result<(FftPlan, FftWorkspace), SolverError> {
    Err(SolverError::InvalidDomain)
}

pub(super) fn from_catalog(
    layout: Layout,
    catalog: &FftCatalog,
) -> Result<(FftPlan, FftWorkspace), SolverError> {
    if catalog.backend != FftBackend::RustFft6_4_1AvxFma {
        return Err(SolverError::InvalidPayload);
    }
    let dimensions = validate_layout(layout)?;
    let forward = [
        catalog.plan(dimensions[0], FftDirection::Forward)?,
        catalog.plan(dimensions[1], FftDirection::Forward)?,
        catalog.plan(dimensions[2], FftDirection::Forward)?,
    ];
    let inverse = [
        catalog.plan(dimensions[0], FftDirection::Inverse)?,
        catalog.plan(dimensions[1], FftDirection::Inverse)?,
        catalog.plan(dimensions[2], FftDirection::Inverse)?,
    ];
    from_parts(layout, forward, inverse)
}

fn from_parts(
    layout: Layout,
    forward: [Arc<dyn Fft<f64>>; 3],
    inverse: [Arc<dyn Fft<f64>>; 3],
) -> Result<(FftPlan, FftWorkspace), SolverError> {
    let maximum = layout
        .dimensions()
        .into_iter()
        .max()
        .ok_or(SolverError::InvalidDomain)?;
    let required = forward
        .iter()
        .chain(&inverse)
        .map(|plan| plan.get_inplace_scratch_len())
        .max()
        .ok_or(SolverError::InvalidDomain)?;
    if required > AVX_SCRATCH_LANES * maximum {
        return Err(SolverError::ResourceLimit);
    }
    let workspace = FftWorkspace {
        layout,
        grid: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        input: filled(maximum, Complex64::new(0.0, 0.0))?,
        output: filled(maximum, Complex64::new(0.0, 0.0))?,
        scratch: filled(
            (AVX_SCRATCH_LANES + TRANSVERSE_TILE_LANES)
                .checked_mul(maximum)
                .ok_or(SolverError::SizeOverflow)?,
            Complex64::new(0.0, 0.0),
        )?,
    };
    Ok((
        FftPlan {
            layout,
            backend: BackendPlan::Avx(boxed_axes(forward, inverse)?),
        },
        workspace,
    ))
}

fn validate_layout(layout: Layout) -> Result<[usize; 3], SolverError> {
    let dimensions = layout.dimensions();
    if dimensions.iter().any(|n| !LENGTHS.contains(n)) {
        return Err(SolverError::InvalidDomain);
    }
    Ok(dimensions)
}

#[cfg(target_arch = "x86_64")]
fn make_catalog() -> Result<FftCatalog, SolverError> {
    require_features()?;
    let mut planner =
        rustfft::FftPlannerAvx::<f64>::new().map_err(|_| SolverError::InvalidDomain)?;
    let mut avx = Vec::new();
    avx.try_reserve_exact(LENGTHS.len())
        .map_err(|_| SolverError::AllocationFailed)?;
    for length in LENGTHS {
        avx.push(CatalogEntry {
            length,
            forward: planner.plan_fft(length, FftDirection::Forward),
            inverse: planner.plan_fft(length, FftDirection::Inverse),
        });
    }
    Ok(FftCatalog {
        backend: FftBackend::RustFft6_4_1AvxFma,
        avx,
    })
}

#[cfg(not(target_arch = "x86_64"))]
fn make_catalog() -> Result<FftCatalog, SolverError> {
    Err(SolverError::InvalidDomain)
}

#[cfg(target_arch = "x86_64")]
fn require_features() -> Result<(), SolverError> {
    if std::is_x86_feature_detected!("avx")
        && std::is_x86_feature_detected!("avx2")
        && std::is_x86_feature_detected!("fma")
    {
        Ok(())
    } else {
        Err(SolverError::InvalidDomain)
    }
}

#[cfg(target_arch = "x86_64")]
pub(super) fn ensure_available() -> Result<(), SolverError> {
    require_features()
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn ensure_available() -> Result<(), SolverError> {
    Err(SolverError::InvalidDomain)
}

fn boxed_axes(
    forward: [Arc<dyn Fft<f64>>; 3],
    inverse: [Arc<dyn Fft<f64>>; 3],
) -> Result<Box<[Axes]>, SolverError> {
    let mut owner = Vec::new();
    owner
        .try_reserve_exact(1)
        .map_err(|_| SolverError::AllocationFailed)?;
    owner.push(Axes { forward, inverse });
    Ok(owner.into_boxed_slice())
}
