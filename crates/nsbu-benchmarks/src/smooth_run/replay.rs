//! Bounded actual from-rest replay of a reconstruction checkpoint's complete canonical bytes.
//!
//! Matching replay establishes reproducibility under this executable's fixed smooth provider,
//! not PDE convergence or authentication of a claimed compiler/force/reference artifact.
use super::{reconstructed_archive, ReconstructedPlan, ReconstructedRun};
use crate::smooth_observer::reconstruction::ReconstructionObserver;
use nsbu_solver::{checkpoint::CheckpointError, SolverError};
use sha2::{Digest, Sha256};

/// Replay admission, integration or canonical-byte comparison failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    /// Caller did not admit every recorded attempt; no partial replay is accepted.
    AttemptLimit,
    /// Complete canonical run bytes differ after actual independent from-rest evolution.
    DifferentReplay,
    /// Numerical/resource admission or a replayed operation failed.
    Numerical(SolverError),
    /// Versioned checkpoint encoding refused the supplied state/history.
    Checkpoint(CheckpointError),
}
impl From<SolverError> for ReplayError {
    fn from(e: SolverError) -> Self {
        Self::Numerical(e)
    }
}
impl From<CheckpointError> for ReplayError {
    fn from(e: CheckpointError) -> Self {
        Self::Checkpoint(e)
    }
}

/// Complete simultaneous storage and finite replay work, with the borrowed original included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayBounds {
    /// Original owner, fresh owner, two canonical byte buffers and fixed replay metadata.
    pub storage_bytes: usize,
    /// Exact recorded attempts to reproduce, including rejected/refused attempts.
    pub attempts: usize,
    /// Bytes in each compared canonical reconstruction archive.
    pub archive_bytes: usize,
    /// Upper byte visits for two encodings, component/outer hashes, comparison and report hash.
    pub archive_byte_visits: usize,
    /// Conservative integration RHS allowance from the original complete configuration.
    pub integration_calls: usize,
    /// Complete integration and independent observation provider allowance.
    pub provider_work_units: usize,
    /// Integration plus observation scalar transforms under the complete configuration.
    pub scalar_transforms: usize,
    /// Independent accepted-derivative coefficient visits under the observation allowance.
    pub reconstruction_modal_visits: usize,
}
/// Evidence of byte-for-byte reproduction, scoped to the current executable and fixed smooth case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayReport {
    /// Digest of the exact complete canonical bytes that were compared, not an external authority.
    pub archive_sha256: [u8; 32],
    /// Every successful, rejected or refused recorded attempt reproduced.
    pub attempts: usize,
    /// Full compared byte count, including physical state, raw history, work and accepted nodes.
    pub compared_bytes: usize,
}
/// Independently integrated result. Its state was evolved from rest during this verification.
pub struct VerifiedReplay {
    run: ReconstructedRun,
    report: ReplayReport,
}
impl VerifiedReplay {
    /// Read the independently evolved state/history without altering the borrowed original.
    pub fn run(&self) -> &ReconstructedRun {
        &self.run
    }
    /// Exact reproduction evidence, with no convergence or external-artifact acceptance status.
    pub fn report(&self) -> ReplayReport {
        self.report
    }
    /// Continue the newly integrated run with its original remaining allowances.
    pub fn into_run(self) -> ReconstructedRun {
        self.run
    }
}
/// Immutable original borrow prevents checkpoint mutation between admission and comparison.
pub struct ReplayPlan<'a> {
    original: &'a ReconstructedRun,
    bounds: ReplayBounds,
}
impl<'a> ReplayPlan<'a> {
    /// Require explicit attempt and aggregate memory caps before any replay allocation.
    /// The cap includes the original owner, even though it is already allocated by the caller.
    pub fn new(
        original: &'a ReconstructedRun,
        maximum_attempts: usize,
        cap: usize,
    ) -> Result<Self, ReplayError> {
        let attempts = original.history.controller().attempted();
        if attempts > maximum_attempts {
            return Err(ReplayError::AttemptLimit);
        }
        let plan = ReconstructedPlan::from_rest(
            original.state.plan().domain(),
            original.initial_clock,
            original.configuration,
            original.observer_samples,
            original.advective_limit,
            cap,
        )?;
        let bounds = reservation(original, plan, attempts)?;
        if bounds.storage_bytes > cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self { original, bounds })
    }
    /// All storage and finite integration/diagnostic work admitted before execution.
    pub fn bounds(&self) -> ReplayBounds {
        self.bounds
    }
    /// Evolve a fresh independent state from rest and compare every canonical archive byte.
    /// Both encodings present the original current diagnostic-origin tag; that tag is a
    /// declaration, not a numerical result. The independently evolved run keeps its own origin.
    /// The original remains unchanged and keeps its original unverified or internal origin.
    pub fn execute(self) -> Result<VerifiedReplay, ReplayError> {
        let source = self.original;
        let mut expected = bytes(self.bounds.archive_bytes)?;
        let mut actual = bytes(self.bounds.archive_bytes)?;
        reconstructed_archive::write(source, &mut expected)?;
        let mut run = ReconstructedRun::from_rest(
            source.state.plan().domain(),
            source.initial_clock,
            source.configuration,
            source.observer_samples,
            source.advective_limit,
            source.state.plan().total(),
        )?;
        for _ in 0..self.bounds.attempts {
            run.step()?;
        }
        if reconstructed_archive::encoded_len(&run)? != expected.len() {
            return Err(ReplayError::DifferentReplay);
        }
        reconstructed_archive::write_with_origin(&run, &mut actual, source.origin)?;
        if actual != expected {
            return Err(ReplayError::DifferentReplay);
        }
        let report = ReplayReport {
            archive_sha256: Sha256::digest(&actual).into(),
            attempts: self.bounds.attempts,
            compared_bytes: actual.len(),
        };
        Ok(VerifiedReplay { run, report })
    }
}
fn reservation(
    source: &ReconstructedRun,
    plan: ReconstructedPlan,
    attempts: usize,
) -> Result<ReplayBounds, ReplayError> {
    let archive_bytes = reconstructed_archive::encoded_len(source)?;
    let storage_bytes = source
        .state
        .plan()
        .total()
        .checked_add(plan.resources().total())
        .and_then(|n| n.checked_add(archive_bytes.checked_mul(2)?))
        .and_then(|n| {
            n.checked_add(
                std::mem::size_of::<VerifiedReplay>() + std::mem::size_of::<ReplayPlan<'_>>(),
            )
        })
        .ok_or(SolverError::SizeOverflow)?;
    let observer =
        ReconstructionObserver::limits(plan.resources().domain(), plan.observer_samples())?;
    Ok(ReplayBounds {
        storage_bytes,
        attempts,
        archive_bytes,
        archive_byte_visits: archive_bytes
            .checked_mul(8)
            .ok_or(SolverError::SizeOverflow)?,
        integration_calls: plan.integration_calls(),
        provider_work_units: plan
            .integration_work_units()
            .checked_add(observer.work_units)
            .ok_or(SolverError::SizeOverflow)?,
        scalar_transforms: plan
            .integration_scalar_transforms()
            .checked_add(observer.scalar_transforms)
            .ok_or(SolverError::SizeOverflow)?,
        reconstruction_modal_visits: observer.modal_visits,
    })
}
fn bytes(n: usize) -> Result<Vec<u8>, SolverError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    bytes.resize(n, 0);
    Ok(bytes)
}
