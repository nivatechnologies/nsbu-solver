use crate::model::{debug, NodeBinding, ProbePlan, Snapshot};
use nsbu_solver::{domain::validate_spectrum, Complex64};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

const MAGIC: &[u8; 12] = b"P10AVXSNAP1\0";
const TRAILER_BYTES: usize = 32;
const IO_BUFFER_BYTES: usize = 8 * 1024 * 1024;
const MAX_PLAN_BYTES: u64 = 64 * 1024;
const MAX_INPUT_BYTES: u64 = 64 * 1024;
const MAX_IDENTITY_BYTES: usize = 16 * 1024;

pub(crate) fn read_plan(path: &Path) -> Result<ProbePlan, String> {
    let bytes = read_bounded(path, MAX_INPUT_BYTES, "probe plan exceeds 64 KiB")?;
    let mut plan: ProbePlan = serde_json::from_slice(&bytes).map_err(debug)?;
    if plan.frozen_plan.is_relative() {
        plan.frozen_plan = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&plan.frozen_plan);
    }
    crate::validate_plan(&plan)?;
    verify_file_hash(&plan.frozen_plan, &plan.frozen_plan_sha256, MAX_PLAN_BYTES)?;
    Ok(plan)
}

pub(crate) fn load(
    plan: &ProbePlan,
    binding: &NodeBinding,
    snapshot_root: &Path,
) -> Result<Snapshot, String> {
    let path = resolve(snapshot_root, &binding.snapshot)?;
    let expected = expected_file_len(plan)?;
    if fs::metadata(&path).map_err(debug)?.len() != expected as u64 {
        return Err("snapshot length mismatch".into());
    }
    let file = File::open(path).map_err(debug)?;
    let mut reader = HashReader::new(BufReader::with_capacity(IO_BUFFER_BYTES, file));
    if reader.array::<12>()? != *MAGIC {
        return Err("snapshot magic mismatch".into());
    }
    let identity_len = usize::try_from(reader.u64()?).map_err(debug)?;
    if identity_len != plan.snapshot_identity.len() || identity_len > MAX_IDENTITY_BYTES {
        return Err("snapshot identity length mismatch".into());
    }
    let mut identity = vec![0_u8; identity_len];
    reader.exact(&mut identity)?;
    if identity != plan.snapshot_identity.as_bytes() {
        return Err("snapshot identity mismatch".into());
    }
    let header = [
        reader.u128()?,
        reader.u128()?,
        reader.u128()?,
        reader.u128()?,
    ];
    if header
        != [
            binding.clock,
            plan.clock_target,
            binding.epoch,
            binding.accepted_steps,
        ]
    {
        return Err("snapshot clock binding mismatch".into());
    }
    let half = plan.domain()?.layout().half_len();
    let mut coefficient_hash = Sha256::new();
    let mut coefficients = std::array::from_fn(|_| Vec::with_capacity(half));
    for component in &mut coefficients {
        for _ in 0..half {
            let bytes = reader.array::<16>()?;
            coefficient_hash.update(bytes);
            component.push(Complex64::new(
                f64::from_bits(u64::from_le_bytes(bytes[..8].try_into().map_err(debug)?)),
                f64::from_bits(u64::from_le_bytes(bytes[8..].try_into().map_err(debug)?)),
            ));
        }
    }
    let coefficient_digest = coefficient_hash.finalize();
    if reader.array::<TRAILER_BYTES>()?.as_slice() != coefficient_digest.as_slice() {
        return Err("coefficient SHA-256 trailer mismatch".into());
    }
    let file_digest = reader.finish()?;
    let coefficient_sha256 = format!("{coefficient_digest:x}");
    let file_sha256 = format!("{file_digest:x}");
    if coefficient_sha256 != binding.coefficient_sha256 || file_sha256 != binding.file_sha256 {
        return Err("snapshot hash does not match frozen node binding".into());
    }
    for component in &coefficients {
        validate_spectrum(plan.domain()?.layout(), component, 1e-12).map_err(debug)?;
    }
    Ok(Snapshot { coefficients })
}

fn resolve(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("snapshot path must be relative and confined".into());
    }
    Ok(root.join(relative))
}

fn expected_file_len(plan: &ProbePlan) -> Result<usize, String> {
    plan.domain()?
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .and_then(|value| value.checked_add(MAGIC.len() + 8 + plan.snapshot_identity.len()))
        .and_then(|value| value.checked_add(4 * 16 + TRAILER_BYTES))
        .ok_or_else(|| "snapshot length overflow".into())
}

fn verify_file_hash(path: &Path, expected: &str, cap: u64) -> Result<(), String> {
    let file = File::open(path).map_err(debug)?;
    let mut reader = file.take(cap + 1);
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer).map_err(debug)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        hash.update(&buffer[..count]);
    }
    if total > cap || format!("{:x}", hash.finalize()) != expected {
        return Err("frozen plan SHA-256 mismatch".into());
    }
    Ok(())
}

fn read_bounded(path: &Path, cap: u64, message: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(debug)?;
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes).map_err(debug)?;
    if bytes.len() as u64 > cap {
        return Err(message.into());
    }
    Ok(bytes)
}

struct HashReader<R> {
    inner: R,
    hash: Sha256,
}

impl<R: Read> HashReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hash: Sha256::new(),
        }
    }

    fn exact(&mut self, output: &mut [u8]) -> Result<(), String> {
        self.inner.read_exact(output).map_err(debug)?;
        self.hash.update(output);
        Ok(())
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let mut output = [0_u8; N];
        self.exact(&mut output)?;
        Ok(output)
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn u128(&mut self) -> Result<u128, String> {
        Ok(u128::from_le_bytes(self.array()?))
    }

    fn finish(mut self) -> Result<impl std::fmt::LowerHex, String> {
        let mut byte = [0_u8; 1];
        if self.inner.read(&mut byte).map_err(debug)? != 0 {
            return Err("snapshot trailing bytes".into());
        }
        Ok(self.hash.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_paths_are_confined_to_the_supplied_root() {
        let root = Path::new("/tmp/frozen-root");
        assert_eq!(
            resolve(root, Path::new("clock-0896/state.bin")).unwrap(),
            root.join("clock-0896/state.bin")
        );
        assert!(resolve(root, Path::new("../state.bin")).is_err());
        assert!(resolve(root, Path::new("/tmp/state.bin")).is_err());
    }
}
