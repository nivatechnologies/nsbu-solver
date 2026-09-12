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
