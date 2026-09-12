use sha2::{Digest, Sha256};
use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

const N: usize = 384;
const COMPONENTS: usize = 3;
const COMPLEX_BYTES: usize = 16;
const BUFFER_BYTES: usize = 8 * 1024 * 1024;
const MAGIC: &[u8] = b"P10N384STEP1\0";
const IDENTITY: &str =
    "prepared-zero-field;n=384;components=3;schema=p10-n384-step-snapshot-v1;integrated=false";
const METADATA_ALLOWANCE: usize = 2 * 4096;
const FILESYSTEM_ALLOWANCE: usize = 64 * 1024;
const CAP_128_GIB: usize = 128 * 1024 * 1024 * 1024;
const CAP_256_GIB: usize = 256 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
struct IoCounters {
    read_bytes: u64,
    write_bytes: u64,
}

#[derive(Debug)]
struct Measurement {
    write_sync_seconds: f64,
    rename_sync_seconds: f64,
    validate_seconds: f64,
    coefficient_bytes: usize,
    file_bytes: u64,
    digest: String,
    io: IoCounters,
}

#[derive(Clone, Copy, Debug)]
struct Admission {
    coefficient_bytes: usize,
    snapshot_bytes: usize,
    per_step: usize,
    steps_64: usize,
    steps_128: usize,
}

struct BundlePaths {
    root: PathBuf,
    partial: PathBuf,
    published: PathBuf,
}

#[derive(Debug)]
enum Command {
    Preflight,
    Benchmark(PathBuf),
}

fn main() -> io::Result<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    run(parse(&arguments)?)
}

fn parse(arguments: &[String]) -> io::Result<Command> {
    match arguments {
        [mode] if mode == "preflight" => Ok(Command::Preflight),
        [mode, path] if mode == "benchmark" => Ok(Command::Benchmark(PathBuf::from(path))),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "usage")),
    }
}

fn run(command: Command) -> io::Result<()> {
    match command {
        Command::Preflight => report_preflight(),
        Command::Benchmark(path) => benchmark(&path, coefficient_bytes(N)?).map(report_measurement),
    }
}

fn report_preflight() -> io::Result<()> {
    let admitted = admission()?;
    println!(
        "preflight identity={IDENTITY} coefficient_bytes={} snapshot_bytes={} per_step_bound={} steps64_bytes={} steps128_bytes={} cap128gib_bytes={CAP_128_GIB} cap256gib_bytes={CAP_256_GIB} steps64_cap128_admitted={} steps128_cap128_admitted={} steps128_cap256_admitted={}",
        admitted.coefficient_bytes,
        admitted.snapshot_bytes,
        admitted.per_step,
        admitted.steps_64,
        admitted.steps_128,
        admitted.steps_64 <= CAP_128_GIB,
        admitted.steps_128 <= CAP_128_GIB,
        admitted.steps_128 <= CAP_256_GIB,
    );
    Ok(())
}

fn admission() -> io::Result<Admission> {
    let coefficient_bytes = coefficient_bytes(N)?;
    let snapshot_bytes = snapshot_bytes(coefficient_bytes)?;
    let per_step = per_step_bound(snapshot_bytes)?;
    Ok(Admission {
        coefficient_bytes,
        snapshot_bytes,
        per_step,
        steps_64: total_bound(per_step, 64)?,
        steps_128: total_bound(per_step, 128)?,
    })
}

fn report_measurement(measurement: Measurement) {
    println!(
        "measurement field=prepared_zero_not_integrated n={N} coefficient_bytes={} file_bytes={} write_sync_seconds={:.9} rename_directory_sync_seconds={:.9} validate_seconds={:.9} digest={} proc_read_bytes={} proc_write_bytes={} hash_validated=true atomic_publish=true bulk_removed_after_validation=true",
        measurement.coefficient_bytes,
        measurement.file_bytes,
        measurement.write_sync_seconds,
        measurement.rename_sync_seconds,
        measurement.validate_seconds,
        measurement.digest,
        measurement.io.read_bytes,
        measurement.io.write_bytes,
    );
}

fn coefficient_bytes(n: usize) -> io::Result<usize> {
    n.checked_mul(n)
        .and_then(|value| value.checked_mul(n / 2 + 1))
        .and_then(|value| value.checked_mul(COMPONENTS * COMPLEX_BYTES))
        .ok_or_else(overflow)
}

fn snapshot_bytes(coefficients: usize) -> io::Result<usize> {
    MAGIC
        .len()
        .checked_add(8 + IDENTITY.len() + 4 * 16)
        .and_then(|value| value.checked_add(coefficients))
        .and_then(|value| value.checked_add(32))
        .ok_or_else(overflow)
}

fn per_step_bound(snapshot: usize) -> io::Result<usize> {
    snapshot
        .checked_add(METADATA_ALLOWANCE + FILESYSTEM_ALLOWANCE)
        .ok_or_else(overflow)
}

fn total_bound(per_step: usize, steps: usize) -> io::Result<usize> {
    per_step.checked_mul(steps).ok_or_else(overflow)
}

fn overflow() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "size overflow")
}

fn benchmark(root: &Path, coefficient_bytes: usize) -> io::Result<Measurement> {
    let paths = prepare_bundle(root)?;
    let io_before = io_counters()?;
    let (digest, write_sync_seconds) = stage_bundle(&paths, coefficient_bytes)?;
    let rename_sync_seconds = publish_bundle(&paths)?;
    let (validate_seconds, file_bytes) = validate_bundle(&paths, coefficient_bytes, &digest)?;
    let io = io_counters()?.difference(io_before);
    cleanup(&paths)?;
    Ok(Measurement {
        write_sync_seconds,
        rename_sync_seconds,
        validate_seconds,
        coefficient_bytes,
        file_bytes,
        digest,
        io,
    })
}

fn prepare_bundle(root: &Path) -> io::Result<BundlePaths> {
    refuse_existing(root)?;
    fs::create_dir(root)?;
    let partial = root.join("step-0001.partial");
    let published = root.join("step-0001");
    fs::create_dir(&partial)?;
    Ok(BundlePaths {
        root: root.to_path_buf(),
        partial,
        published,
    })
}

fn stage_bundle(paths: &BundlePaths, coefficient_bytes: usize) -> io::Result<(String, f64)> {
    let write_started = Instant::now();
    let digest = write_snapshot(&paths.partial.join("state.bin"), coefficient_bytes)?;
    write_synced(
        &paths.partial.join("attempt.json"),
        b"{\"outcome\":\"committed\"}\n",
    )?;
    write_synced(
        &paths.partial.join("record.json"),
        b"{\"observation_status\":\"NotScheduled\"}\n",
    )?;
    sync_directory(&paths.partial)?;
    Ok((digest, write_started.elapsed().as_secs_f64()))
}

fn publish_bundle(paths: &BundlePaths) -> io::Result<f64> {
    let rename_started = Instant::now();
    fs::rename(&paths.partial, &paths.published)?;
    sync_directory(&paths.root)?;
    Ok(rename_started.elapsed().as_secs_f64())
}

fn validate_bundle(
    paths: &BundlePaths,
    coefficient_bytes: usize,
    digest: &str,
) -> io::Result<(f64, u64)> {
    let snapshot = paths.published.join("state.bin");
    let validate_started = Instant::now();
    validate_snapshot(&snapshot, coefficient_bytes, digest)?;
    let validate_seconds = validate_started.elapsed().as_secs_f64();
    let file_bytes = fs::metadata(&snapshot)?.len();
    Ok((validate_seconds, file_bytes))
}

fn cleanup(paths: &BundlePaths) -> io::Result<()> {
    fs::remove_dir_all(&paths.published)?;
    sync_directory(&paths.root)?;
    fs::remove_dir(&paths.root)
}

fn refuse_existing(path: &Path) -> io::Result<()> {
    if path.exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "output exists",
        ))
    } else {
        Ok(())
    }
}

fn write_snapshot(path: &Path, coefficient_bytes: usize) -> io::Result<String> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut writer = BufWriter::with_capacity(BUFFER_BYTES, file);
    write_header(&mut writer)?;
    let digest = write_zero_coefficients(&mut writer, coefficient_bytes)?;
    writer.write_all(&digest)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(hex(&digest))
}

fn write_header(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(MAGIC)?;
    writer.write_all(&(IDENTITY.len() as u64).to_le_bytes())?;
    writer.write_all(IDENTITY.as_bytes())?;
    for value in [0_u128, 4096, 0, 0] {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_zero_coefficients(
    writer: &mut impl Write,
    coefficient_bytes: usize,
) -> io::Result<[u8; 32]> {
    let chunk = vec![0_u8; BUFFER_BYTES.min(coefficient_bytes)];
    let mut remaining = coefficient_bytes;
    let mut hash = Sha256::new();
    while remaining > 0 {
        let count = remaining.min(chunk.len());
        writer.write_all(&chunk[..count])?;
        hash.update(&chunk[..count]);
        remaining -= count;
    }
    Ok(hash.finalize().into())
}

fn validate_snapshot(path: &Path, coefficient_bytes: usize, expected: &str) -> io::Result<()> {
    let mut reader = BufReader::with_capacity(BUFFER_BYTES, File::open(path)?);
    validate_header(&mut reader)?;
    let actual = hash_bytes(&mut reader, coefficient_bytes)?;
    let mut stored = [0_u8; 32];
    reader.read_exact(&mut stored)?;
    let mut tail = [0_u8; 1];
    if reader.read(&mut tail)? != 0 || actual.as_slice() != stored || hex(&actual) != expected {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "hash mismatch"));
    }
    Ok(())
}

fn validate_header(reader: &mut impl Read) -> io::Result<()> {
    let mut header = vec![0_u8; MAGIC.len() + 8 + IDENTITY.len() + 4 * 16];
    reader.read_exact(&mut header)?;
    let identity_start = MAGIC.len() + 8;
    if &header[..MAGIC.len()] != MAGIC
        || &header[identity_start..identity_start + IDENTITY.len()] != IDENTITY.as_bytes()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "header mismatch",
        ));
    }
    Ok(())
}

fn hash_bytes(reader: &mut impl Read, bytes: usize) -> io::Result<[u8; 32]> {
    let mut buffer = vec![0_u8; BUFFER_BYTES.min(bytes)];
    let mut remaining = bytes;
    let mut hash = Sha256::new();
    while remaining > 0 {
        let count = remaining.min(buffer.len());
        reader.read_exact(&mut buffer[..count])?;
        hash.update(&buffer[..count]);
        remaining -= count;
    }
    Ok(hash.finalize().into())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut output = String::with_capacity(2 * bytes.len());
    for &byte in bytes {
        output.push(DIGITS[usize::from(byte >> 4)] as char);
        output.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    output
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn io_counters() -> io::Result<IoCounters> {
    let text = fs::read_to_string("/proc/self/io")?;
    Ok(IoCounters {
        read_bytes: counter(&text, "read_bytes:")?,
        write_bytes: counter(&text, "write_bytes:")?,
    })
}

fn counter(text: &str, key: &str) -> io::Result<u64> {
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|value| value.trim().parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "proc io"))
}

impl IoCounters {
    fn difference(self, earlier: Self) -> Self {
        Self {
            read_bytes: self.read_bytes.saturating_sub(earlier.read_bytes),
            write_bytes: self.write_bytes.saturating_sub(earlier.write_bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_layout_and_cap_admission() {
        let admitted = admission().unwrap();
        assert_eq!(admitted.coefficient_bytes, 1_366_032_384);
        assert!(admitted.steps_64 <= CAP_128_GIB);
        assert!(admitted.steps_128 > CAP_128_GIB);
        assert!(admitted.steps_128 <= CAP_256_GIB);
    }

    #[test]
    fn tiny_snapshot_round_trip_and_corruption_refusal() {
        let root = temp_path("round-trip");
        fs::create_dir(&root).unwrap();
        let path = root.join("state.bin");
        let digest = write_snapshot(&path, 257).unwrap();
        validate_snapshot(&path, 257, &digest).unwrap();
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[1]).unwrap();
        assert_eq!(
            validate_snapshot(&path, 257, &digest).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_output_is_refused() {
        let root = temp_path("existing");
        fs::create_dir(&root).unwrap();
        assert_eq!(
            refuse_existing(&root).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn tiny_transaction_publishes_validates_and_cleans_up() {
        let root = temp_path("transaction");
        let measured = benchmark(&root, 257).unwrap();
        assert_eq!(measured.coefficient_bytes, 257);
        assert_eq!(measured.file_bytes, snapshot_bytes(257).unwrap() as u64);
        assert!(!root.exists());
    }

    #[test]
    fn command_parser_binds_mode_and_refuses_incomplete_input() {
        assert!(matches!(
            parse(&["preflight".into()]).unwrap(),
            Command::Preflight
        ));
        assert!(matches!(
            parse(&["benchmark".into(), "path".into()]).unwrap(),
            Command::Benchmark(_)
        ));
        assert_eq!(parse(&[]).unwrap_err().kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            parse(&["unknown".into()]).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            parse(&["unknown".into(), "path".into()])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!("p10-n384-snapshot-{label}-{}", std::process::id()))
    }
}
