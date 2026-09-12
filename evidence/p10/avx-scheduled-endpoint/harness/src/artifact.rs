//! Harness-owned transactional snapshot and record publication; no resume decoder exists.
use nsbu_solver::{diagnostics::balances::BalanceSample, domain::SpectralState, SolverError};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

pub const BUFFER_BYTES: usize = 1024 * 1024;
pub const DISK_CAP_BYTES: usize = 4 * 1024 * 1024 * 1024;
const HEADER_ALLOWANCE: usize = 4096;

pub struct NodeRecord<'a> {
    pub identity: &'a str,
    pub balance: BalanceSample,
    pub observer_seconds: f64,
    pub force_seconds: f64,
    pub conservative_seconds: f64,
    pub transfer_measure_seconds: f64,
}

pub fn disk_preflight(state_bytes: usize, nodes: usize) -> Result<usize, SolverError> {
    state_bytes
        .checked_add(HEADER_ALLOWANCE)
        .and_then(|n| n.checked_mul(nodes))
        .and_then(|n| n.checked_add(128 * HEADER_ALLOWANCE))
        .filter(|&n| n <= DISK_CAP_BYTES)
        .ok_or(SolverError::ResourceLimit)
}

pub fn publish_node(
    root: &Path,
    state: &SpectralState,
    record: NodeRecord<'_>,
) -> io::Result<String> {
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
    let result = publish_node_inner(&partial, state, record);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&partial);
        return Err(error);
    }
    File::open(&partial)?.sync_all()?;
    fs::rename(&partial, &final_path)?;
    File::open(root)?.sync_all()?;
    result
}

fn publish_node_inner(
    partial: &Path,
    state: &SpectralState,
    record: NodeRecord<'_>,
) -> io::Result<String> {
    let (hash, bytes) = write_snapshot(&partial.join("state.bin"), state, record.identity)?;
    let json = node_json(state, record, &hash, bytes);
    write_file(&partial.join("record.json"), json.as_bytes())?;
    Ok(hash)
}

fn write_snapshot(
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

fn node_json(
    state: &SpectralState,
    record: NodeRecord<'_>,
    hash: &str,
    coefficient_bytes: usize,
) -> String {
    let balance = record.balance;
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-scheduled-node-v1\",\n",
            "  \"identity\": \"{}\",\n  \"resumable\": false,\n",
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
        record.identity,
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

pub fn publish_attempt(root: &Path, index: usize, json: &str) -> io::Result<()> {
    atomic_file(root, &format!("attempt-{index:03}.json"), json.as_bytes())
}

pub fn publish_status(root: &Path, name: &str, json: &str) -> io::Result<()> {
    atomic_file(root, name, json.as_bytes())
}

fn atomic_file(root: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    atomic_file_with(root, name, |path| write_file(path, bytes))
}

fn atomic_file_with(
    root: &Path,
    name: &str,
    write: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let final_path = root.join(name);
    let partial = partial_path(root, name);
    if final_path.exists() || partial.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "artifact path already exists",
        ));
    }
    if let Err(error) = write(&partial) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    fs::rename(&partial, &final_path)?;
    File::open(root)?.sync_all()
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn create(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn partial_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.partial"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn atomic_file_refuses_overwrite_and_leaves_no_partial_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("p10-endpoint-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        publish_status(&root, "status.json", "first").unwrap();
        assert_eq!(
            publish_status(&root, "status.json", "second")
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read_to_string(root.join("status.json")).unwrap(),
            "first"
        );
        assert!(!root.join("status.json.partial").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_write_publishes_nothing_and_removes_partial_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("p10-endpoint-fail-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let result = atomic_file_with(&root, "node.json", |path| {
            write_file(path, b"incomplete")?;
            Err(io::Error::other("injected after write"))
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
        assert!(!root.join("node.json").exists());
        assert!(!root.join("node.json.partial").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disk_preflight_refuses_more_than_the_fixed_cap() {
        assert_eq!(
            disk_preflight(DISK_CAP_BYTES, 2),
            Err(SolverError::ResourceLimit)
        );
    }
}
