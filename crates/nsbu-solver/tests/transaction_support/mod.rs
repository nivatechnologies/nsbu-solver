//! Shared setup for independently owned transactional test payloads.
use nsbu_solver::domain::{Epoch, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::{attempt::AttemptWorkspace, transaction::CandidateState};

pub fn setup(
    plan: ResourcePlan,
    clock: TickClock,
) -> (SpectralState, CandidateState, AttemptWorkspace) {
    (
        SpectralState::from_rest(plan, clock, Epoch(0)).unwrap(),
        CandidateState::new(plan, clock, Epoch(0)).unwrap(),
        AttemptWorkspace::new(plan).unwrap(),
    )
}
