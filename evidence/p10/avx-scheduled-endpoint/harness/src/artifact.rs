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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationKind {
    Attempt,
    Node,
}

pub struct StagedArtifact {
    partial: PathBuf,
    final_path: PathBuf,
    root: PathBuf,
    kind: PublicationKind,
    state_hash: Option<String>,
    published: bool,
    preserve_on_failure: bool,
}

impl StagedArtifact {
    pub fn publish(mut self) -> io::Result<PublishedArtifact> {
        self.preserve_on_failure = true;
        fs::rename(&self.partial, &self.final_path)?;
        File::open(&self.root)?.sync_all()?;
        self.published = true;
        Ok(PublishedArtifact {
            kind: self.kind,
            state_hash: self.state_hash.take(),
        })
    }
}

impl Drop for StagedArtifact {
    fn drop(&mut self) {
        if !self.published && !self.preserve_on_failure {
            let _ = if self.partial.is_dir() {
                fs::remove_dir_all(&self.partial)
            } else {
                fs::remove_file(&self.partial)
            };
        }
    }
}

#[derive(Debug)]
pub struct PublishedArtifact {
    pub kind: PublicationKind,
    pub state_hash: Option<String>,
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
    let published = stage_node(root, state, record, None)?.publish()?;
    published
        .state_hash
        .ok_or_else(|| io::Error::other("node publication omitted state hash"))
}

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
    if let Err(error) = File::open(&partial)?.sync_all() {
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
    stage_attempt(root, index, json)?.publish().map(|_| ())
}

pub fn stage_attempt(root: &Path, index: usize, json: &str) -> io::Result<StagedArtifact> {
    let name = format!("attempt-{index:03}.json");
    let final_path = root.join(&name);
    let partial = partial_path(root, &name);
    refuse_existing(&final_path, &partial)?;
    if let Err(error) = write_file(&partial, json.as_bytes()) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    Ok(StagedArtifact {
        partial,
        final_path,
        root: root.to_owned(),
        kind: PublicationKind::Attempt,
        state_hash: None,
        published: false,
        preserve_on_failure: false,
    })
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
    refuse_existing(&final_path, &partial)?;
    if let Err(error) = write(&partial) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    fs::rename(&partial, &final_path)?;
    File::open(root)?.sync_all()
}

fn refuse_existing(final_path: &Path, partial: &Path) -> io::Result<()> {
    if final_path.exists() || partial.exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "artifact path already exists",
        ))
    } else {
        Ok(())
    }
}

pub fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value <= '\u{1f}' => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", value as u32).expect("String writes cannot fail");
            }
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
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
    use nsbu_solver::domain::{
        Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock,
    };
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

    #[test]
    fn json_string_escapes_quotes_newlines_and_control_characters() {
        assert_eq!(
            json_string("quoted \"line\"\nslash\\tab\t\u{1}"),
            "\"quoted \\\"line\\\"\\nslash\\\\tab\\t\\u0001\""
        );
    }

    #[test]
    fn scheduled_node_stages_and_publishes_snapshot_record_and_attempt_together() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "p10-endpoint-bundle-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let plan = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force: 0,
                diagnostics: 0,
                overhead: 0,
            },
            1024 * 1024,
            Epoch(0),
        )
        .unwrap();
        let state =
            SpectralState::from_rest(plan, TickClock::from_rest(-20, 8192).unwrap(), Epoch(0))
                .unwrap();
        let staged = stage_node(
            &root,
            &state,
            NodeRecord {
                identity: "test",
                balance: BalanceSample::REST,
                observer_seconds: 0.0,
                force_seconds: 0.0,
                conservative_seconds: 0.0,
                transfer_measure_seconds: 0.0,
            },
            Some("{\"outcome\":\"committed\"}\n"),
        )
        .unwrap();
        let partial = root.join("node-0000.partial");
        assert!(partial.join("state.bin").exists());
        assert!(partial.join("record.json").exists());
        assert!(partial.join("attempt.json").exists());
        assert!(!root.join("node-0000").exists());
        staged.publish().unwrap();
        let published = root.join("node-0000");
        assert!(published.join("state.bin").exists());
        assert!(published.join("record.json").exists());
        assert!(published.join("attempt.json").exists());
        assert!(!partial.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
