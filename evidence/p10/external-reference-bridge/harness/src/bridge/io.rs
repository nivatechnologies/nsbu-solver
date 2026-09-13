use super::*;
use std::{fs::File, io::Read};

pub(super) fn resolve(manifest: &Path, value: &mut PathBuf) {
    if value.is_relative() {
        *value = manifest
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&*value);
    }
}

pub(super) fn verify_file(
    path: &Path,
    cap: u64,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    let bytes = read_bounded(path, cap, &format!("{label} exceeds byte bound"))?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(format!("{label} SHA-256 mismatch"));
    }
    Ok(())
}

pub(super) fn verify_current_executable(expected: &str) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(debug)?;
    verify_file(&executable, u64::MAX - 1, expected, "bridge executable")
}

pub(super) fn read_bounded(path: &Path, cap: u64, message: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(debug)?;
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes).map_err(debug)?;
    if bytes.len() as u64 > cap {
        return Err(message.into());
    }
    Ok(bytes)
}
