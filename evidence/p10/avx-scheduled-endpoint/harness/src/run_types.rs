//! Harness-owner values carried across proposal staging and publication.
use crate::{artifact::StagedArtifact, balance::TimedBalance, records};
use nsbu_solver::integrators::transaction::AcceptedAttempt;

pub struct AcceptedStage {
    pub artifact: StagedArtifact,
    pub balance: Option<TimedBalance>,
    pub timing: Option<records::ObservationTiming>,
}

pub struct AcceptedFacts {
    pub token: AcceptedAttempt,
    pub facts: records::AttemptFacts,
}

#[derive(Clone, Copy)]
pub struct StageMeta {
    pub balance: Option<TimedBalance>,
    pub timing: Option<records::ObservationTiming>,
}

use crate::{
    cache::CachedReducedForce, observer::ReducedObserver, publication::Frontiers,
    timed_rhs::TimedRhs,
};
use nsbu_solver::{
    domain::{ResourcePlan, SpectralState},
    integrators::{attempt::AttemptWorkspace, rhs::SpectralRhs, transaction::CandidateState},
};

pub struct RunOwners {
    pub resources: ResourcePlan,
    pub state: SpectralState,
    pub candidate: CandidateState,
    pub attempts: AttemptWorkspace,
    pub rhs: TimedRhs<SpectralRhs<CachedReducedForce>>,
    pub observer: ReducedObserver,
    pub identity: String,
    pub balances: Vec<TimedBalance>,
    pub frontiers: Frontiers,
}
