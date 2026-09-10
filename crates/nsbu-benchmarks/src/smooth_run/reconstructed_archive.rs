//! Coherent external reconstruction checkpoints with immutable unverified import origin.
//!
//! The outer checksum binds a common physical/history/work frame and accepted reconstruction
//! nodes. The common frame uses the version-one byte layout, but its reconstruction resource
//! profile and initial observation charge make it invalid as a balance-only archive.
use super::{
    archive::{self, Cursor, ImportedSmoothRun},
    Origin, ReconstructedRun,
};
use crate::smooth_observer::reconstruction::{archive as nodes, ReconstructionObserver};
use nsbu_solver::{
    checkpoint::CheckpointError,
    domain::{ResourcePlan, SpectralState},
    experiment::{control::Configuration, log::RunHistory},
    SolverError,
};
use sha2::{Digest, Sha256};
const MAGIC: &[u8; 8] = b"NSBURC01";
const HEADER: usize = 42;
const HASH: usize = 32;

/// Decoded, structurally coherent reconstruction run with no authenticated provenance.
#[derive(Debug)]
pub struct ImportedReconstructedRun {
    core: ImportedSmoothRun,
    nodes: nodes::UnverifiedReconstruction,
}
impl ImportedReconstructedRun {
    /// Every decoded payload remains externally unverified, regardless of encoded origin.
    pub fn origin(&self) -> Origin {
        Origin::ExternalUnverified
    }
    /// Read-only physical state bound to the accepted-node and raw-history payloads.
    pub fn state(&self) -> &SpectralState {
        self.core.state()
    }
    /// Frozen decoded execution configuration.
    pub fn configuration(&self) -> Configuration {
        self.core.configuration()
    }
    /// Replayed raw history; balances and refusals retain their original order.
    pub fn history(&self) -> &RunHistory {
        self.core.history()
    }
    /// Continue from imported physical bits, accepted reconstruction and spent budgets.
    pub fn continue_unverified(self, cap: usize) -> Result<ReconstructedRun, SolverError> {
        self.core
            .continue_observed::<ReconstructionObserver>(self.nodes.into_snapshot(), cap)
    }
}
/// Exact bytes for the two payloads and their complete integrity frame, without allocation.
pub fn encoded_len(run: &ReconstructedRun) -> Result<usize, CheckpointError> {
    framed_len(archive::core_len(run)?, nodes::encoded_len(run.observer())?)
}
/// Maximum archive size for a bounded reconstruction profile, without allocating a run.
pub fn maximum_encoded_len(
    plan: ResourcePlan,
    configuration: Configuration,
) -> Result<usize, CheckpointError> {
    framed_len(
        archive::maximum_encoded_len(plan, configuration)?,
        nodes::maximum_encoded_len(plan.domain())?,
    )
}
fn framed_len(core: usize, nodes: usize) -> Result<usize, CheckpointError> {
    HEADER
        .checked_add(core)
        .and_then(|size| size.checked_add(nodes))
        .and_then(|size| size.checked_add(HASH))
        .ok_or(CheckpointError::ResourceLimit)
}
/// Encode the complete reconstruction-enabled owner into caller-owned storage.
/// Short buffers are rejected before modification; no numerical evaluation is performed.
pub fn write(run: &ReconstructedRun, output: &mut [u8]) -> Result<usize, CheckpointError> {
    write_with_origin(run, output, run.origin)
}
// Replay presents both encodings with the same diagnostic tag; it never edits either run origin.
pub(super) fn write_with_origin(
    run: &ReconstructedRun,
    output: &mut [u8],
    origin: Origin,
) -> Result<usize, CheckpointError> {
    let core = archive::core_len(run)?;
    let nodes = nodes::encoded_len(run.observer())?;
    let required = framed_len(core, nodes)?;
    if output.len() < required {
        return Err(CheckpointError::ResourceLimit);
    }
    let mut p = 0;
    archive::put(output, &mut p, MAGIC);
    archive::put(output, &mut p, &1u16.to_le_bytes());
    for size in [core, nodes] {
        archive::put(output, &mut p, &(size as u128).to_le_bytes());
    }
    p += archive::write_core_with_origin(run, &mut output[p..p + core], origin)?;
    p += nodes::write(run.observer(), &mut output[p..p + nodes])?;
    let hash = Sha256::digest(&output[..p]);
    archive::put(output, &mut p, &hash);
    Ok(p)
}
/// Verify bounded bytes, replay raw history and bind every retained node to actual run clocks.
/// Caps include peak decoded storage and fresh execution scratch; input bytes remain borrowed.
pub fn read(
    bytes: &[u8],
    expected: ResourcePlan,
    maximum_bytes: usize,
    storage_cap: usize,
) -> Result<ImportedReconstructedRun, CheckpointError> {
    let (core_bytes, node_bytes) = split_payloads(bytes, maximum_bytes)?;
    let extra = ReconstructionObserver::snapshot_reservation(expected.domain())
        .map_err(CheckpointError::InvalidHistory)?
        .checked_add(std::mem::size_of::<ImportedReconstructedRun>())
        .ok_or(CheckpointError::ResourceLimit)?;
    let core_cap = storage_cap
        .checked_sub(extra)
        .ok_or(CheckpointError::ResourceLimit)?;
    let core = archive::admission::read_profile::<ReconstructionObserver>(
        core_bytes,
        expected,
        core_bytes.len(),
        core_cap,
        1,
    )?;
    let nodes = nodes::read(
        node_bytes,
        expected,
        core.observer_samples,
        core.state(),
        node_bytes.len(),
        extra,
    )?;
    nodes.validate_owner(core.history(), core.observer_work)?;
    Ok(ImportedReconstructedRun { core, nodes })
}
fn split_payloads(bytes: &[u8], maximum: usize) -> Result<(&[u8], &[u8]), CheckpointError> {
    if bytes.len() > maximum {
        return Err(CheckpointError::ResourceLimit);
    }
    if bytes.len() < HEADER + HASH {
        return Err(CheckpointError::InvalidEncoding);
    }
    let (body, hash) = bytes.split_at(bytes.len() - HASH);
    if Sha256::digest(body).as_slice() != hash {
        return Err(CheckpointError::HashMismatch);
    }
    let mut c = Cursor { remaining: body };
    if c.take(8)? != MAGIC || u16::from_le_bytes(c.array()?) != 1 {
        return Err(CheckpointError::InvalidEncoding);
    }
    let core = archive::size(&mut c)?;
    let nodes = archive::size(&mut c)?;
    if framed_len(core, nodes)? != bytes.len() {
        return Err(CheckpointError::InvalidEncoding);
    }
    Ok((c.take(core)?, c.take(nodes)?))
}
