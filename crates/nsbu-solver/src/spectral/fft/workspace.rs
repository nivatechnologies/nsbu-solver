//! Workspace identity, shape, scratch, and finite-value validation.
use super::{BackendPlan, FftBackend, FftPlan, FftWorkspace};
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
        let required_tile = maximum
            .checked_mul(super::TRANSVERSE_TILE_LANES)
            .ok_or(SolverError::SizeOverflow)?;
        if real != self.layout.real_len()
            || half != self.layout.half_len()
            || work.layout != self.layout
            || work.grid.len() != self.layout.half_len()
            || work.input.len() != maximum
            || work.output.len() != maximum
            || work.scratch.len() < required_scratch
            || work.transverse_tile.len() != required_tile
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
