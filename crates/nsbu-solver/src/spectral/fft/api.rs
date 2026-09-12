//! Public plan admission, construction, and backend identity.
use super::{avx, owned, BackendPlan, FftBackend, FftCatalog, FftPlan, FftWorkspace};
use crate::{domain::Layout, SolverError};

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
            FftBackend::OwnedRadix => owned::reservation(layout),
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
            FftBackend::OwnedRadix => owned::new(layout),
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
            FftBackend::OwnedRadix => owned::reservation(layout),
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
            FftBackend::OwnedRadix => owned::new(layout),
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
}
