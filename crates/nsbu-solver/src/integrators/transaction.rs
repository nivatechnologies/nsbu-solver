//! Private acceptance identities and allocation-free, all-or-nothing payload exchange.
use crate::domain::{Epoch, ResourcePlan, SpectralState, TickClock};
use crate::SolverError;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Stamp {
    plan: ResourcePlan,
    clock: TickClock,
    epoch: Epoch,
    pointers: [usize; 3],
    capacities: [usize; 3],
    lengths: [usize; 3],
    digest: u64,
    count: u128,
}

fn stamp(state: &SpectralState) -> Stamp {
    // Fingerprint supplements private storage identity and epochs; not a cryptographic problem ID.
    let mut digest = 0xcbf29ce484222325_u64;
    for component in &state.components {
        for value in component {
            for bits in [value.re.to_bits(), value.im.to_bits()] {
                digest = (digest ^ bits).wrapping_mul(0x100000001b3);
            }
        }
    }
    Stamp {
        plan: state.plan,
        clock: state.clock,
        epoch: state.epoch,
        pointers: std::array::from_fn(|i| state.components[i].as_ptr() as usize),
        capacities: std::array::from_fn(|i| state.components[i].capacity()),
        lengths: std::array::from_fn(|i| state.components[i].len()),
        digest,
        count: state.accepted_steps,
    }
}

/// Independently owned candidate payload. Contents and acceptance metadata are private.
#[derive(Debug)]
pub struct CandidateState {
    pub(crate) state: SpectralState,
    generation: Epoch,
    accepted: Option<Epoch>,
}

impl CandidateState {
    /// Allocate separate buffers under the same approved reservation as the trajectory.
    pub fn new(plan: ResourcePlan, clock: TickClock, epoch: Epoch) -> Result<Self, SolverError> {
        Ok(Self {
            state: SpectralState::from_rest(plan, clock, epoch)?,
            generation: Epoch(0),
            accepted: None,
        })
    }

    pub(crate) fn invalidate(&mut self) -> Result<(), SolverError> {
        self.accepted = None;
        self.generation = self.generation.next()?;
        Ok(())
    }

    pub(crate) fn accept(&mut self, base: &SpectralState) -> AcceptedAttempt {
        self.accepted = Some(self.generation);
        AcceptedAttempt {
            base: stamp(base),
            candidate: stamp(&self.state),
            generation: self.generation,
        }
    }
}

/// Single-use permission issued only after a successful numerical attempt. Not Clone.
#[derive(Debug)]
pub struct AcceptedAttempt {
    base: Stamp,
    candidate: Stamp,
    generation: Epoch,
}

/// Validate every identity before exchanging any payload. Consumes the acceptance token.
/// Candidate retains the previous committed storage for the next attempt; no buffer is dropped.
pub fn commit_candidate(
    plan: ResourcePlan,
    committed: &mut SpectralState,
    candidate: &mut CandidateState,
    accepted: AcceptedAttempt,
) -> Result<(), SolverError> {
    if plan != committed.plan
        || plan != candidate.state.plan
        || accepted.base != stamp(committed)
        || accepted.candidate != stamp(&candidate.state)
        || candidate.generation != accepted.generation
        || candidate.accepted != Some(accepted.generation)
    {
        return Err(SolverError::StaleAttempt);
    }
    std::mem::swap(committed, &mut candidate.state);
    candidate.accepted = None;
    Ok(())
}

#[cfg(test)]
#[path = "transaction_tests.rs"]
mod tests;
