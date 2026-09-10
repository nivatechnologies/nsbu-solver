use nsbu_benchmarks::smooth_run::archive::ImportedSmoothRun;
use nsbu_solver::{domain::TickClock, experiment::control::Configuration, SolverError};
use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    path::Path,
};

use crate::ADVECTIVE_LIMIT;

pub(crate) fn matches_profile(
    imported: &ImportedSmoothRun,
    expected: Configuration,
    clock: TickClock,
    samples: usize,
) -> bool {
    let found = imported.configuration();
    found.method == expected.method
        && found.limits.endpoint == expected.limits.endpoint
        && found.limits.step_ticks == expected.limits.step_ticks
        && found.limits.maximum_attempts == expected.limits.maximum_attempts
        && found.tolerances.absolute.map(f64::to_bits)
            == expected.tolerances.absolute.map(f64::to_bits)
        && found.tolerances.relative.map(f64::to_bits)
            == expected.tolerances.relative.map(f64::to_bits)
        && imported.initial_clock() == clock
        && imported.observer_samples() == samples
        && imported.advective_limit_bits() == ADVECTIVE_LIMIT.to_bits()
}

pub(crate) fn bounded_buffer(size: usize) -> Result<Vec<u8>, SolverError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| SolverError::AllocationFailed)?;
    bytes.resize(size, 0);
    Ok(bytes)
}

pub(crate) fn read_checkpoint(
    path: &Path,
    maximum: usize,
    cap: usize,
    plan_bytes: usize,
) -> Result<Vec<u8>, &'static str> {
    let length = admitted_length(path, maximum, cap, plan_bytes)?;
    let mut bytes = bounded_buffer(length).map_err(|_| "checkpoint_read_allocation_failed")?;
    let mut file = File::open(path).map_err(|_| "checkpoint_open_failed")?;
    read_exact_contents(&mut file, &mut bytes)?;
    Ok(bytes)
}

fn read_exact_contents(file: &mut impl Read, bytes: &mut [u8]) -> Result<(), &'static str> {
    file.read_exact(bytes)
        .map_err(|_| "checkpoint_read_failed")?;
    let mut extra = [0_u8; 1];
    if file
        .read(&mut extra)
        .map_err(|_| "checkpoint_read_failed")?
        != 0
    {
        return Err("checkpoint_file_changed");
    }
    Ok(())
}

fn admitted_length(
    path: &Path,
    maximum: usize,
    cap: usize,
    plan_bytes: usize,
) -> Result<usize, &'static str> {
    let metadata = fs::metadata(path).map_err(|_| "checkpoint_metadata_failed")?;
    if !metadata.is_file() {
        return Err("checkpoint_not_regular_file");
    }
    let length = metadata.len();
    let length = usize::try_from(length).map_err(|_| "checkpoint_too_large")?;
    if length > maximum
        || plan_bytes
            .checked_add(length)
            .filter(|total| *total <= cap)
            .is_none()
    {
        return Err("checkpoint_too_large");
    }
    Ok(length)
}

pub(crate) fn publish_checkpoint(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let parent = parent(path);
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("checkpoint");
    for attempt in 0..16_u8 {
        let temporary = parent.join(format!(".{stem}.nsbu-{}-{attempt}.tmp", std::process::id()));
        let mut file = match create_temporary(&temporary) {
            Ok(file) => file,
            Err("checkpoint_temp_collision") => continue,
            Err(error) => return Err(error),
        };
        let result = write_temporary(&mut file, bytes);
        drop(file);
        let result = result.and_then(|()| link_output(&temporary, path));
        if result.is_err() {
            cleanup(&temporary);
            return result;
        }
        cleanup(&temporary);
        return sync_parent(parent);
    }
    Err("checkpoint_temp_exhausted")
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn create_temporary(path: &Path) -> Result<File, &'static str> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => Ok(file),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Err("checkpoint_temp_collision"),
        Err(_) => Err("checkpoint_temp_create_failed"),
    }
}

fn write_temporary(file: &mut File, bytes: &[u8]) -> Result<(), &'static str> {
    file.write_all(bytes)
        .map_err(|_| "checkpoint_write_failed")?;
    file.sync_all().map_err(|_| "checkpoint_sync_failed")
}

fn link_output(temporary: &Path, output: &Path) -> Result<(), &'static str> {
    match fs::hard_link(temporary, output) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Err("checkpoint_output_exists"),
        Err(_) => Err("checkpoint_publish_failed"),
    }
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);
}

fn sync_parent(parent: &Path) -> Result<(), &'static str> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "checkpoint_published_durability_unconfirmed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_reader_rejects_shrinking_and_growing_contents() {
        let mut bytes = [0; 2];
        assert_eq!(
            read_exact_contents(&mut std::io::Cursor::new(b"ab"), &mut bytes),
            Ok(())
        );
        assert_eq!(bytes, *b"ab");
        assert_eq!(
            read_exact_contents(&mut std::io::Cursor::new(b"a"), &mut bytes),
            Err("checkpoint_read_failed")
        );
        assert_eq!(
            read_exact_contents(&mut std::io::Cursor::new(b"abc"), &mut bytes),
            Err("checkpoint_file_changed")
        );
    }

    #[test]
    fn collision_is_preserved_and_the_next_name_is_used() {
        let directory = std::env::temp_dir().join(format!("nsbu-cli-io-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let output = directory.join("saved.bin");
        let sentinel = directory.join(format!(".saved.bin.nsbu-{}-0.tmp", std::process::id()));
        fs::write(&sentinel, b"sentinel").unwrap();
        publish_checkpoint(&output, b"checkpoint").unwrap();
        assert_eq!(fs::read(&sentinel).unwrap(), b"sentinel");
        assert_eq!(fs::read(&output).unwrap(), b"checkpoint");
        fs::remove_file(sentinel).unwrap();
        fs::remove_file(output).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
