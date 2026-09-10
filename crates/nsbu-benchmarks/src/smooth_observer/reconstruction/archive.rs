//! Bounded binary accepted-node data. Byte decoding never authenticates trajectory provenance.
use super::{snapshot::ReconstructionSnapshot, Node, ReconstructionObserver};
use crate::smooth_observer::BalanceObserverWork;
use nsbu_solver::{
    checkpoint::CheckpointError,
    domain::{Domain, ResourcePlan, SpectralState, TickClock},
    SolverError,
};
mod codec;
mod read;
use codec::{put, write_node};
const MAGIC: &[u8; 8] = b"NSBURN01";
const HEADER: usize = 107;

/// Externally supplied accepted nodes whose structural checks are not a provenance proof.
#[derive(Debug)]
pub struct UnverifiedReconstruction {
    snapshot: ReconstructionSnapshot,
}
impl UnverifiedReconstruction {
    /// Restore independent scratch while retaining the imported nodes and spent work.
    /// This cannot inject a trusted snapshot into an owned from-rest run.
    pub fn restore_unverified(
        self,
        plan: ResourcePlan,
        samples: usize,
        state: &SpectralState,
    ) -> Result<ReconstructionObserver, SolverError> {
        ReconstructionObserver::restore(plan, samples, state, self.snapshot)
    }
    pub(crate) fn validate_owner(
        &self,
        history: &nsbu_solver::experiment::log::RunHistory,
        work: BalanceObserverWork,
    ) -> Result<(), CheckpointError> {
        if self.snapshot.work != work {
            return Err(CheckpointError::InvalidEncoding);
        }
        let step = history.controller().configuration().limits.step_ticks;
        for node in &self.snapshot.accepted[..self.snapshot.accepted_count] {
            let elapsed = node
                .steps
                .checked_mul(step)
                .ok_or(CheckpointError::ResourceLimit)?;
            if node.clock.is_none_or(|clock| clock.elapsed() != elapsed) {
                return Err(CheckpointError::InvalidHistory(SolverError::InvalidClock));
            }
        }
        Ok(())
    }
    pub(crate) fn into_snapshot(self) -> ReconstructionSnapshot {
        self.snapshot
    }
}

/// Exact encoded byte count without allocation. A pending proposal cannot be saved.
pub fn encoded_len(observer: &ReconstructionObserver) -> Result<usize, CheckpointError> {
    if observer.has_pending {
        return Err(CheckpointError::InvalidHistory(SolverError::StaleAttempt));
    }
    encoded_size(observer.source, observer.accepted_count)
}
/// Maximum component size for a complete three-node ring on the declared domain.
pub fn maximum_encoded_len(domain: Domain) -> Result<usize, CheckpointError> {
    encoded_size(domain, 3)
}
fn encoded_size(domain: Domain, count: usize) -> Result<usize, CheckpointError> {
    if !(1..=3).contains(&count) {
        return Err(CheckpointError::InvalidEncoding);
    }
    domain
        .layout()
        .half_len()
        .checked_mul(96)
        .and_then(|n| n.checked_add(84))
        .and_then(|n| n.checked_mul(count))
        .and_then(|n| n.checked_add(HEADER))
        .ok_or(CheckpointError::ResourceLimit)
}
/// Serialize accepted fields and finite work counters into caller-owned storage.
/// A short buffer is refused before any output byte changes; floating-point bits are preserved.
pub fn write(
    observer: &ReconstructionObserver,
    output: &mut [u8],
) -> Result<usize, CheckpointError> {
    let required = encoded_len(observer)?;
    if output.len() < required {
        return Err(CheckpointError::ResourceLimit);
    }
    let mut p = 0;
    put(output, &mut p, MAGIC);
    put(output, &mut p, &1u16.to_le_bytes());
    put(
        output,
        &mut p,
        &(observer.limits.samples as u128).to_le_bytes(),
    );
    put(output, &mut p, &[observer.accepted_count as u8]);
    let work = observer.balance.consumption();
    for value in [
        work.samples,
        work.work_units,
        work.scalar_transforms,
        observer.modal_visits,
        observer.source.layout().half_len(),
    ] {
        put(output, &mut p, &(value as u128).to_le_bytes());
    }
    debug_assert_eq!(p, HEADER);
    for node in &observer.accepted[..observer.accepted_count] {
        write_node(node, output, &mut p);
    }
    Ok(p)
}

/// Decode only under an independently approved plan, state, sample bound and complete byte cap.
/// This component has no checksum; an owning archive must bind it to the physical/history bytes.
pub fn read(
    bytes: &[u8],
    plan: ResourcePlan,
    samples: usize,
    state: &SpectralState,
    maximum_bytes: usize,
    storage_cap: usize,
) -> Result<UnverifiedReconstruction, CheckpointError> {
    read::decode(bytes, plan, samples, state, maximum_bytes, storage_cap)
}
