//! Bounded input decode for the N256/M512 pair adapter.
//!
//! The full-state byte layout is bound byte-for-byte to the reviewed Rust
//! writer schema in the avx-scheduled-endpoint harness
//! (`artifact/snapshot.rs::write_snapshot`): magic `P10AVXSNAP1\0`, u64
//! little-endian identity length, identity bytes, four little-endian u128
//! clock words (elapsed, target, epoch, accepted_steps), three strict
//! half-spectrum components as consecutive little-endian `f64` bit patterns
//! (real then imaginary), and a 32-byte coefficient SHA-256 trailer over the
//! coefficient payload. The whole-file SHA-256 is an additional binding, never
//! part of the file itself.
use crate::model::{debug, ClockHeader, Manifest, Snapshot, INPUT_SCHEMA};
use nsbu_solver::{domain::validate_spectrum, Complex64};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

const MAGIC: &[u8; 12] = b"P10AVXSNAP1\0";
const TRAILER_BYTES: usize = 32;
const IO_BUFFER_BYTES: usize = 8 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_IDENTITY_BYTES: usize = 16 * 1024;
const FIXED_OVERHEAD_BYTES: usize = 1024 * 1024;
const MAX_PLAN_BYTES: u64 = 1024 * 1024;

pub(crate) fn read_manifest(path: &Path) -> Result<Manifest, String> {
    let bytes = read_bounded(path, MAX_MANIFEST_BYTES, "manifest exceeds 64 KiB bound")?;
    let mut manifest: Manifest = serde_json::from_slice(&bytes).map_err(debug)?;
    validate_envelope(&manifest)?;
    validate_hashes(&manifest)?;
    if manifest.snapshot.is_relative() {
        manifest.snapshot = beside(path, &manifest.snapshot);
    }
    if manifest.plan.is_relative() {
        manifest.plan = beside(path, &manifest.plan);
    }
    if let Some(binding) = &mut manifest.lineage {
        if binding.intake.is_relative() {
            binding.intake = beside(path, &binding.intake);
        }
    }
    manifest.domain()?;
    verify_plan(&manifest)?;
    Ok(manifest)
}

fn validate_envelope(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema != INPUT_SCHEMA
        || manifest.identity.len() > MAX_IDENTITY_BYTES
        || manifest.backend.is_empty()
        || manifest.execution.is_empty()
    {
        return Err("invalid pair comparison manifest binding".into());
    }
    let guard = &manifest.admission_guard;
    if !guard.advective_limit.is_finite()
        || guard.advective_limit <= 0.0
        || guard.maximum_attempts == 0
    {
        return Err("invalid admission guard metadata".into());
    }
    if !manifest.profile.matches_identity(&manifest.identity) {
        return Err("exact profile does not match snapshot identity".into());
    }
    if manifest.evolution.clock_target != manifest.target
        || manifest.evolution.comparison_endpoint != manifest.elapsed
    {
        return Err("manifest clock envelope mismatch".into());
    }
    Ok(())
}

fn validate_hashes(manifest: &Manifest) -> Result<(), String> {
    if !is_hex(&manifest.source_commit, 40)
        || !is_hex(&manifest.plan_sha256, 64)
        || !is_hex(&manifest.coefficient_sha256, 64)
        || !is_hex(&manifest.file_sha256, 64)
        || !is_hex(&manifest.evolution.case_sha256, 64)
    {
        return Err("invalid pair comparison manifest binding".into());
    }
    if let Some(binding) = &manifest.lineage {
        if !is_hex(&binding.intake_sha256, 64) {
            return Err("invalid lineage intake hash binding".into());
        }
    }
    validate_evolution(manifest)
}

fn validate_evolution(manifest: &Manifest) -> Result<(), String> {
    let evolution = &manifest.evolution;
    if evolution.method != "cox-matthews"
        || evolution.integration_force_dimensions != [512; 3]
        || evolution.schedule.is_empty()
    {
        return Err("invalid pair evolution semantics".into());
    }
    validate_tolerances(evolution)?;
    validate_schedule(evolution)
}

fn validate_tolerances(evolution: &crate::model::Evolution) -> Result<(), String> {
    let values = evolution
        .absolute_tolerances
        .into_iter()
        .chain(evolution.relative_tolerances);
    if values
        .into_iter()
        .any(|value| !value.is_finite() || value <= 0.0)
    {
        return Err("invalid evolution tolerances".into());
    }
    Ok(())
}

fn validate_schedule(evolution: &crate::model::Evolution) -> Result<(), String> {
    let mut next = 0_u128;
    for segment in &evolution.schedule {
        if segment.from_inclusive != next
            || segment.until_exclusive <= segment.from_inclusive
            || segment.step_ticks == 0
            || !(segment.until_exclusive - segment.from_inclusive)
                .is_multiple_of(segment.step_ticks)
        {
            return Err("invalid piecewise schedule".into());
        }
        next = segment.until_exclusive;
    }
    if next != evolution.comparison_endpoint {
        return Err("piecewise schedule does not reach target".into());
    }
    Ok(())
}

pub(crate) fn state_bytes(manifest: &Manifest) -> Result<usize, String> {
    manifest
        .domain()?
        .layout()
        .half_len()
        .checked_mul(3 * 16)
        .ok_or_else(|| "snapshot size overflow".into())
}

pub(crate) fn admitted_bytes(left: &Manifest, right: &Manifest) -> Result<usize, String> {
    state_bytes(left)?
        .checked_add(state_bytes(right)?)
        .and_then(|value| value.checked_add(FIXED_OVERHEAD_BYTES))
        .ok_or_else(|| "resource bound overflow".into())
}

pub(crate) fn preflight(left: &Manifest, right: &Manifest) -> Result<usize, String> {
    check_file_len(left)?;
    check_file_len(right)?;
    admitted_bytes(left, right)
}

pub(crate) fn expected_file_len(manifest: &Manifest) -> Result<usize, String> {
    MAGIC
        .len()
        .checked_add(8)
        .and_then(|value| value.checked_add(manifest.identity.len()))
        .and_then(|value| value.checked_add(4 * 16))
        .and_then(|value| value.checked_add(state_bytes(manifest).ok()?))
        .and_then(|value| value.checked_add(TRAILER_BYTES))
        .ok_or_else(|| "snapshot file size overflow".into())
}

pub(crate) fn check_file_len(manifest: &Manifest) -> Result<(), String> {
    let expected_file = expected_file_len(manifest)?;
    let actual_file = fs::metadata(&manifest.snapshot).map_err(debug)?.len();
    if actual_file != expected_file as u64 {
        return Err("snapshot length mismatch".into());
    }
    Ok(())
}

pub(crate) fn load(manifest: &Manifest) -> Result<Snapshot, String> {
    check_file_len(manifest)?;
    let file = File::open(&manifest.snapshot).map_err(debug)?;
    let mut reader = HashReader::new(BufReader::with_capacity(IO_BUFFER_BYTES, file));
    let clock = read_prefix(&mut reader, manifest)?;
    let layout = manifest.domain()?.layout();
    let (coefficients, coefficient_hash) = decode_components(&mut reader, layout.half_len())?;
    finish_snapshot(reader, manifest, clock, coefficients, coefficient_hash)
}

fn read_prefix<R: Read>(
    reader: &mut HashReader<R>,
    manifest: &Manifest,
) -> Result<ClockHeader, String> {
    if reader.array::<12>()? != *MAGIC {
        return Err("snapshot magic mismatch".into());
    }
    let identity_len = usize::try_from(reader.u64()?).map_err(debug)?;
    if identity_len != manifest.identity.len() || identity_len > MAX_IDENTITY_BYTES {
        return Err("snapshot identity length mismatch".into());
    }
    let mut identity = vec![0_u8; identity_len];
    reader.exact(&mut identity)?;
    if identity != manifest.identity.as_bytes() {
        return Err("snapshot identity mismatch".into());
    }
    let clock = ClockHeader {
        elapsed: reader.u128()?,
        target: reader.u128()?,
        epoch: reader.u128()?,
        accepted_steps: reader.u128()?,
    };
    if clock != ClockHeader::from_manifest(manifest) {
        return Err("snapshot clock mismatch".into());
    }
    Ok(clock)
}

fn finish_snapshot<R: Read>(
    mut reader: HashReader<R>,
    manifest: &Manifest,
    clock: ClockHeader,
    coefficients: [Vec<Complex64>; 3],
    coefficient_hash: Sha256,
) -> Result<Snapshot, String> {
    let coefficient_digest = coefficient_hash.finalize();
    let trailer = reader.array::<TRAILER_BYTES>()?;
    if trailer.as_slice() != coefficient_digest.as_slice() {
        return Err("coefficient SHA-256 trailer mismatch".into());
    }
    let file_digest = reader.finish()?;
    let coefficient_sha256 = format!("{coefficient_digest:x}");
    let file_sha256 = format!("{file_digest:x}");
    if coefficient_sha256 != manifest.coefficient_sha256 || file_sha256 != manifest.file_sha256 {
        return Err("snapshot hash does not match reviewed manifest".into());
    }
    let layout = manifest.domain()?.layout();
    for component in &coefficients {
        validate_spectrum(layout, component, 1e-12).map_err(debug)?;
    }
    Ok(Snapshot {
        coefficients,
        coefficient_sha256,
        file_sha256,
        clock,
    })
}

pub(crate) fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn beside(manifest: &Path, value: &Path) -> std::path::PathBuf {
    manifest
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(value)
}

pub(crate) fn read_bounded(path: &Path, cap: u64, message: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(debug)?;
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes).map_err(debug)?;
    if bytes.len() as u64 > cap {
        return Err(message.into());
    }
    Ok(bytes)
}

fn verify_plan(manifest: &Manifest) -> Result<(), String> {
    verify_bounded_hash(
        &manifest.plan,
        &manifest.plan_sha256,
        "plan exceeds 1 MiB bound",
        "frozen plan SHA-256 mismatch",
    )
}

fn verify_bounded_hash(
    path: &Path,
    expected: &str,
    oversized: &str,
    mismatch: &str,
) -> Result<(), String> {
    let file = File::open(path).map_err(debug)?;
    let mut reader = file.take(MAX_PLAN_BYTES + 1);
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; IO_BUFFER_BYTES];
    loop {
        let count = reader.read(&mut buffer).map_err(debug)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        hash.update(&buffer[..count]);
    }
    if total > MAX_PLAN_BYTES {
        return Err(oversized.into());
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err(mismatch.into());
    }
    Ok(())
}

fn decode_components<R: Read>(
    reader: &mut HashReader<R>,
    component_len: usize,
) -> Result<([Vec<Complex64>; 3], Sha256), String> {
    let mut hash = Sha256::new();
    let mut result = std::array::from_fn(|_| Vec::with_capacity(component_len));
    for component in &mut result {
        for _ in 0..component_len {
            let bytes = reader.array::<16>()?;
            hash.update(bytes);
            let mut re = [0_u8; 8];
            let mut im = [0_u8; 8];
            re.copy_from_slice(&bytes[..8]);
            im.copy_from_slice(&bytes[8..]);
            component.push(Complex64::new(
                f64::from_bits(u64::from_le_bytes(re)),
                f64::from_bits(u64::from_le_bytes(im)),
            ));
        }
    }
    Ok((result, hash))
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
