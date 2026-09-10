//! Physical payload copying only. A complete checkpoint must additionally retain all run history.
use crate::{
    domain::{ResourcePlan, SpectralState},
    storage::filled,
    Complex64, SolverError,
};

/// Independently owned, bit-preserving image of a live state, with no reference-field input.
/// This is not a complete restart checkpoint: controller, balances, error history, artifacts
/// and lineage must be captured by the experiment layer before a restart can be qualified.
#[derive(Debug)]
pub struct PhysicalImage {
    state: SpectralState,
}
impl PhysicalImage {
    /// Extra storage for this image's Fourier payload and inline metadata.
    /// Caller allocator overhead and all other checkpoint records require separate reservations.
    pub fn reservation(plan: ResourcePlan) -> Result<usize, SolverError> {
        plan.domain()
            .layout()
            .half_len()
            .checked_mul(3 * std::mem::size_of::<Complex64>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }
    /// Allocate the admitted extra image before copying; failure cannot alter the live state.
    pub fn capture(state: &SpectralState, extra_cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(state.plan)? > extra_cap {
            return Err(SolverError::ResourceLimit);
        }
        let n = state.plan.domain().layout().half_len();
        let mut components = [
            filled(n, Complex64::new(0.0, 0.0))?,
            filled(n, Complex64::new(0.0, 0.0))?,
            filled(n, Complex64::new(0.0, 0.0))?,
        ];
        for (destination, source) in components.iter_mut().zip(&state.components) {
            destination.copy_from_slice(source);
        }
        Ok(Self {
            state: SpectralState {
                plan: state.plan,
                clock: state.clock,
                epoch: state.epoch,
                components,
                accepted_steps: state.accepted_steps,
            },
        })
    }
    /// Inspect the captured immutable payload; this exposes no mutable coefficient access.
    pub fn state(&self) -> &SpectralState {
        &self.state
    }
    /// Transfer ownership of the reserved payload without allocation, projection or rounding.
    /// A caller must restore matching run history and execution settings before continuation.
    pub fn into_state(self) -> SpectralState {
        self.state
    }
}
