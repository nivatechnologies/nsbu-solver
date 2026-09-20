//! Snapshot byte writer and node-bundle staging shared by every profile.
use super::{create, json_string, NodeRecord, BUFFER_BYTES};
use nsbu_solver::domain::SpectralState;
use sha2::{Digest, Sha256};
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};
#[cfg(not(feature = "n384-prep"))]
use super::{write_file, DISK_CAP_BYTES, PublicationKind, StagedArtifact};
#[cfg(not(feature = "n384-prep"))]
use {nsbu_solver::SolverError, std::fs};

#[cfg(not(feature = "n384-prep"))]
const HEADER_ALLOWANCE: usize = 4096;

#[cfg(not(feature = "n384-prep"))]
pub fn disk_preflight(state_bytes: usize, nodes: usize) -> Result<usize, SolverError> {
    state_bytes
        .checked_add(HEADER_ALLOWANCE)
        .and_then(|n| n.checked_mul(nodes))
        .and_then(|n| n.checked_add(128 * HEADER_ALLOWANCE))
        .filter(|&n| n <= DISK_CAP_BYTES)
        .ok_or(SolverError::ResourceLimit)
}

#[cfg(not(feature = "n384-prep"))]
pub fn publish_node(
    root: &Path,
    state: &SpectralState,
    record: NodeRecord<'_>,
) -> io::Result<String> {
    let published = stage_node(root, state, record, None)?.publish()?;
    published
        .state_hash
        .ok_or_else(|| io::Error::other("node publication omitted state hash"))
}

#[cfg(not(feature = "n384-prep"))]
pub fn stage_node(
    root: &Path,
    state: &SpectralState,
    record: NodeRecord<'_>,
    attempt_json: Option<&str>,
) -> io::Result<StagedArtifact> {
    let clock = state.clock().elapsed();
    let final_path = root.join(format!("node-{clock:04}"));
    let partial = root.join(format!("node-{clock:04}.partial"));
    if final_path.exists() || partial.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "node path already exists",
        ));
    }
    fs::create_dir(&partial)?;
    let result = stage_node_inner(&partial, state, record, attempt_json);
    let hash = match result {
        Ok(hash) => hash,
        Err(error) => {
            let _ = fs::remove_dir_all(&partial);
            return Err(error);
        }
    };
    if let Err(error) = fs::File::open(&partial)?.sync_all() {
        let _ = fs::remove_dir_all(&partial);
        return Err(error);
    }
    Ok(StagedArtifact {
        partial,
        final_path,
        root: root.to_owned(),
        kind: PublicationKind::Node,
        state_hash: Some(hash),
        published: false,
        preserve_on_failure: false,
    })
}

#[cfg(not(feature = "n384-prep"))]
fn stage_node_inner(
    partial: &Path,
    state: &SpectralState,
    record: NodeRecord<'_>,
    attempt_json: Option<&str>,
) -> io::Result<String> {
    let (hash, bytes) = write_snapshot(&partial.join("state.bin"), state, record.identity)?;
    let json = node_json(state, record, &hash, bytes);
    write_file(&partial.join("record.json"), json.as_bytes())?;
    if let Some(attempt) = attempt_json {
        write_file(&partial.join("attempt.json"), attempt.as_bytes())?;
    }
    Ok(hash)
}

pub(crate) fn write_snapshot(
    path: &Path,
    state: &SpectralState,
    identity: &str,
) -> io::Result<(String, usize)> {
    let file = create(path)?;
    let mut writer = BufWriter::with_capacity(BUFFER_BYTES, file);
    writer.write_all(b"P10AVXSNAP1\0")?;
    writer.write_all(&(identity.len() as u64).to_le_bytes())?;
    writer.write_all(identity.as_bytes())?;
    for word in [
        state.clock().elapsed(),
        state.clock().target(),
        state.epoch().0,
        state.accepted_steps(),
    ] {
        writer.write_all(&word.to_le_bytes())?;
    }
    let mut hash = Sha256::new();
    let mut coefficient_bytes = 0usize;
    for axis in 0..3 {
        for value in state
            .component(axis)
            .map_err(|error| io::Error::other(format!("{error:?}")))?
        {
            for word in [value.re.to_bits(), value.im.to_bits()] {
                let bytes = word.to_le_bytes();
                hash.update(bytes);
                writer.write_all(&bytes)?;
                coefficient_bytes += bytes.len();
            }
        }
    }
    let digest = hash.finalize();
    writer.write_all(&digest)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok((format!("{digest:x}"), coefficient_bytes))
}

pub(crate) fn node_json(
    state: &SpectralState,
    record: NodeRecord<'_>,
    hash: &str,
    coefficient_bytes: usize,
) -> String {
    let balance = record.balance;
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-scheduled-node-v1\",\n",
            "  \"identity\": {},\n  \"resumable\": false,\n",
            "  \"clock\": {},\n  \"epoch\": {},\n  \"accepted_steps\": {},\n",
            "  \"coefficient_bytes\": {},\n  \"state_sha256\": \"{}\",\n",
            "  \"timing\": {{\"observer\": {:.9}, \"force\": {:.9}, ",
            "\"conservative\": {:.9}, \"transfer_measure\": {:.9}}},\n",
            "  \"balance\": {{\"l2\": {:.17e}, \"h1\": {:.17e}, ",
            "\"vorticity_l2\": {:.17e}, \"divergence_l2\": {:.17e}, ",
            "\"energy\": {:.17e}, \"enstrophy\": {:.17e}, ",
            "\"energy_dissipation\": {:.17e}, \"forcing_work\": {:.17e}, ",
            "\"stretching\": {:.17e}, \"enstrophy_dissipation\": {:.17e}, ",
            "\"vorticity_forcing\": {:.17e}}}\n}}\n"
        ),
        json_string(record.identity),
        state.clock().elapsed(),
        state.epoch().0,
        state.accepted_steps(),
        coefficient_bytes,
        hash,
        record.observer_seconds,
        record.force_seconds,
        record.conservative_seconds,
        record.transfer_measure_seconds,
        balance.norms.l2,
        balance.norms.h1,
        balance.norms.vorticity_l2,
        balance.norms.divergence_l2,
        balance.energy,
        balance.enstrophy,
        balance.energy_dissipation,
        balance.forcing_work,
        balance.stretching,
        balance.enstrophy_dissipation,
        balance.vorticity_forcing,
    )
}
