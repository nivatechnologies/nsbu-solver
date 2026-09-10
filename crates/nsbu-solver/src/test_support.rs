//! Shared private transactional storage fixture; no numerical oracle is shared.
use crate::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::transaction::CandidateState,
};
pub(crate) fn transaction_setup() -> (ResourcePlan, SpectralState, CandidateState) {
    let plan = ResourcePlan::new(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 0,
            overhead: 4096,
        },
        1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    (
        plan,
        SpectralState::from_rest(plan, clock, Epoch(0)).unwrap(),
        CandidateState::new(plan, clock, Epoch(0)).unwrap(),
    )
}

#[path = "../tests/recorded_config/mod.rs"]
mod recorded_config;
pub(crate) use recorded_config::configuration;
