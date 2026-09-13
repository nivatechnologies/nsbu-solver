//! Public plan admission, construction, and backend identity.
use super::{
    avx, owned, BackendPlan, FftBackend, FftCatalog, FftPlan, FftWorkspace, ParallelAvxPlan,
    ParallelFftExecutor, ParallelFftIdentity,
};
use crate::{domain::Layout, SolverError};
use std::sync::Arc;

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
            BackendPlan::Avx(_) | BackendPlan::ParallelAvx(_) => FftBackend::RustFft6_4_1AvxFma,
        }
    }

    pub(crate) fn with_parallel_executor(
        self,
        executor: Arc<ParallelFftExecutor>,
    ) -> Result<Self, SolverError> {
        if executor.identity().layout != self.layout
            || executor.identity().backend != FftBackend::RustFft6_4_1AvxFma
        {
            return Err(SolverError::InvalidPayload);
        }
        let Self { layout, backend } = self;
        let BackendPlan::Avx(axes) = backend else {
            return Err(SolverError::InvalidPayload);
        };
        let mut axes = axes.into_vec();
        if axes.len() != 1 {
            return Err(SolverError::InvalidPayload);
        }
        let axes_value = axes.pop().ok_or(SolverError::InvalidPayload)?;
        drop(axes);
        Ok(Self {
            layout,
            backend: BackendPlan::ParallelAvx(Box::new(ParallelAvxPlan {
                axes: axes_value,
                executor,
            })),
        })
    }

    pub(crate) fn parallel_wrapper_additional_reservation() -> usize {
        size_of::<ParallelAvxPlan>() - size_of::<avx::Axes>()
    }

    /// Identity of the explicitly attached intra-transform executor, if present.
    pub fn parallel_fft_identity(&self) -> Option<ParallelFftIdentity> {
        match &self.backend {
            BackendPlan::ParallelAvx(owner) => Some(owner.executor.identity()),
            BackendPlan::Owned(_) | BackendPlan::Avx(_) => None,
        }
    }

    pub(crate) fn parallel_executor(&self) -> Option<Arc<ParallelFftExecutor>> {
        match &self.backend {
            BackendPlan::ParallelAvx(owner) => Some(Arc::clone(&owner.executor)),
            BackendPlan::Owned(_) | BackendPlan::Avx(_) => None,
        }
    }

    pub(in crate::spectral::fft) fn avx_axes(&self) -> Option<&avx::Axes> {
        match &self.backend {
            BackendPlan::Avx(axes) => axes.first(),
            BackendPlan::ParallelAvx(owner) => Some(&owner.axes),
            BackendPlan::Owned(_) => None,
        }
    }
}
