//! Attempt-record and status-file publication through the staged rename path.
use super::{
    partial_path, refuse_existing, write_file, PublicationKind, StagedArtifact,
};
use std::{
    fs::{self, File},
    io,
    path::Path,
};

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

pub(crate) fn atomic_file_with(
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
