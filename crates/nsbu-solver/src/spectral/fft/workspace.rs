//! Workspace identity, shape, scratch, and finite-value validation.
use super::{
    BackendPlan, FftBackend, FftPlan, FftWorkspace, AVX_SCRATCH_LANES, TRANSVERSE_TILE_LANES,
};
use crate::{domain::Layout, SolverError};

impl FftPlan {
    pub(crate) fn matches_lane(
        &self,
        layout: Layout,
        backend: FftBackend,
        workspace: &FftWorkspace,
    ) -> bool {
        self.layout == layout
            && self.backend() == backend
            && self
                .validate(layout.real_len(), layout.half_len(), workspace)
                .is_ok()
    }

    pub(super) fn validate(
        &self,
        real: usize,
        half: usize,
        work: &FftWorkspace,
    ) -> Result<(), SolverError> {
        let maximum = self
            .layout
            .dimensions()
            .into_iter()
            .max()
            .ok_or(SolverError::InvalidDomain)?;
        let valid_scratch = match &self.backend {
            BackendPlan::Owned(_) => work.scratch.len() == maximum,
            BackendPlan::Avx(axes) => {
                let required = axes[0]
                    .forward
                    .iter()
                    .chain(&axes[0].inverse)
                    .map(|plan| plan.get_inplace_scratch_len())
                    .max()
                    .ok_or(SolverError::InvalidDomain)?;
                required <= AVX_SCRATCH_LANES * maximum
                    && work.scratch.len() == (AVX_SCRATCH_LANES + TRANSVERSE_TILE_LANES) * maximum
            }
        };
        if real != self.layout.real_len()
            || half != self.layout.half_len()
            || work.layout != self.layout
            || work.grid.len() != self.layout.half_len()
            || work.input.len() != maximum
            || work.output.len() != maximum
            || !valid_scratch
        {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
    }
}

pub(super) fn finite_real(values: &[f64]) -> Result<(), SolverError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}
