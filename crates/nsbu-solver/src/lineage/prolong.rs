//! Explicit coarse-to-fine physical transfer; missing earlier fine evolution is never recovered.
use super::PhysicalImage;
use crate::{
    domain::{ResourcePlan, SpectralState},
    spectral::transfer_validated,
    storage::field,
    SolverError,
};

/// A prolonged physical payload. Its ancestry remains transferred, even if later fine steps pass.
/// Complete restart history and inherited-error reports must accompany this component.
#[derive(Debug)]
pub struct ProlongedState {
    state: SpectralState,
    source_plan: ResourcePlan,
}
impl ProlongedState {
    /// Allocate and copy every source strict-band coefficient without changing its amplitude.
    /// Require identical physical geometry/viscosity and a strictly finer grid with no coarsened
    /// axis. Newly represented coefficients start at zero; no reference evaluator is consulted.
    pub fn from_state(
        source: &SpectralState,
        target: ResourcePlan,
        extra_cap: usize,
    ) -> Result<Self, SolverError> {
        let before = source.plan().domain();
        let after = target.domain();
        let coarse = before.layout().dimensions();
        let fine = after.layout().dimensions();
        if before.lengths() != after.lengths()
            || before.viscosity() != after.viscosity()
            || coarse == fine
            || coarse.into_iter().zip(fine).any(|(a, b)| a > b)
        {
            return Err(SolverError::InvalidDomain);
        }
        if Self::reservation(target)? > extra_cap {
            return Err(SolverError::ResourceLimit);
        }
        let epoch = source.epoch().next()?;
        let n = after.layout().half_len();
        let mut components = field(n)?;
        for (input, output) in source.components.iter().zip(&mut components) {
            transfer_validated(before.layout(), after.layout(), input, output);
        }
        Ok(Self {
            source_plan: source.plan(),
            state: SpectralState {
                plan: target,
                clock: source.clock(),
                epoch,
                components,
                accepted_steps: source.accepted_steps(),
            },
        })
    }
    /// Complete extra physical payload and inline metadata; other restart history is separate.
    pub fn reservation(target: ResourcePlan) -> Result<usize, SolverError> {
        PhysicalImage::reservation(target)?
            .checked_add(std::mem::size_of::<ResourcePlan>())
            .ok_or(SolverError::SizeOverflow)
    }
    /// Original resource/grid identity retained with this physical transfer.
    pub fn source_plan(&self) -> ResourcePlan {
        self.source_plan
    }
    /// Immutable prolonged state; this is not the independently evolved fine reference branch.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }
    /// Move the physical payload without allocating; callers must retain transferred lineage.
    pub fn into_state(self) -> SpectralState {
        self.state
    }
}
