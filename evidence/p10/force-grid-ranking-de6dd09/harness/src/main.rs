use nsbu_benchmarks::{provider::parallel_reduced::ParallelReducedV2Force, CASE_SHA256};
use nsbu_solver::{
    diagnostics::{comparison::ComparisonPlan, norms::Norms},
    domain::{validate_spectrum, Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::{modal, FftBackend, FftCatalog},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

const CAP: usize = 96 * 1024 * 1024 * 1024;
const N: usize = 256;
const WORKERS: usize = 32;
const ARTIFACT_BUFFER_BYTES: usize = 1024 * 1024;
const METADATA_BYTES: usize = 64 * 1024;
const SOURCE_LENGTH: usize = 40;
const CASE_LENGTH: usize = 64;
const MAGIC: [u8; 16] = *b"NSBUFORCEv1\0\0\0\0\0";

struct Spectrum {
    sampled: usize,
    values: [Vec<Complex64>; 3],
}

fn main() -> Result<(), SolverError> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, m, output] if command == "sample" => sample(parse(m)?, output, false),
        [command, m, output, flag] if command == "sample" && flag == "--dry-run" => {
            sample(parse(m)?, output, true)
        }
        [command, left, right] if command == "compare" => compare(left, right),
        _ => Err(SolverError::InvalidPayload),
    }
}

fn sample(m: usize, output_path: &str, dry: bool) -> Result<(), SolverError> {
    if ![384, 512, 576, 768].contains(&m) || env!("RUN_SOURCE").len() != SOURCE_LENGTH {
        return Err(SolverError::InvalidPayload);
    }
    let domain = Domain::new([N; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([m; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)?;
    let output_bytes = domain
        .layout()
        .half_len()
        .checked_mul(3 * std::mem::size_of::<Complex64>())
        .ok_or(SolverError::SizeOverflow)?;
    let peak = catalog_bytes
        .checked_add(limits.storage_bytes)
        .and_then(|value| value.checked_add(2 * output_bytes))
        .and_then(|value| value.checked_add(ARTIFACT_BUFFER_BYTES + METADATA_BYTES))
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} backend=rustfft-6.4.1-avx-avx2-fma retained={N} sampled={m} workers={WORKERS} quantum=-20 target=8192 elapsed=4096 remaining=4096 catalog_bytes={catalog_bytes} force={limits:?} output_bytes={output_bytes} peak_with_previous_spectrum_bytes={peak} cap={CAP} artifact_buffer_bytes={ARTIFACT_BUFFER_BYTES} metadata_bytes={METADATA_BYTES} output={output_path}",
        env!("RUN_SOURCE"),
    );
    if peak > CAP {
        return Err(SolverError::ResourceLimit);
    }
    if dry {
        return Ok(());
    }
    refuse_existing(output_path)?;
    let catalog_started = Instant::now();
    let catalog = FftCatalog::new(backend, catalog_bytes)?;
    let catalog_seconds = catalog_started.elapsed().as_secs_f64();
    let construction_started = Instant::now();
    let mut provider = ParallelReducedV2Force::new_with_catalog(
        domain,
        samples,
        WORKERS,
        &catalog,
        limits.storage_bytes,
    )?;
    let construction_seconds = construction_started.elapsed().as_secs_f64();
    let mut values = output(domain)?;
    let clock = TickClock::restore(-20, 8192, 4096, 4096)?;
    let force_started = Instant::now();
    let work = provider.evaluate(clock, limits, values.each_mut().map(Vec::as_mut_slice))?;
    let force_seconds = force_started.elapsed().as_secs_f64();
    validate(domain, &values)?;
    let coefficient_sha256 = fingerprint(&values);
    let (artifact_sha256, artifact_bytes) = write_artifact(output_path, m, &values)?;
    println!(
        "terminal=complete source={} retained={N} sampled={m} workers={WORKERS} catalog_seconds={catalog_seconds:.9} construction_seconds={construction_seconds:.9} force_seconds={force_seconds:.9} work={work:?} root_iterations={} provider_identity={:?} fft_backend={:?} full_spectrum_valid=true coefficient_sha256={coefficient_sha256} artifact_sha256={artifact_sha256} artifact_bytes={artifact_bytes} output={output_path} accepted_pde_windows=0",
        env!("RUN_SOURCE"),
        provider.last_root_iterations(),
        provider.identity(),
        provider.fft_backend(),
    );
    Ok(())
}

fn compare(left_path: &str, right_path: &str) -> Result<(), SolverError> {
    let left = read_artifact(left_path)?;
    let right = read_artifact(right_path)?;
    if left.sampled >= right.sampled {
        return Err(SolverError::InvalidPayload);
    }
    let domain = Domain::new([N; 3], [1.0; 3], 1.0)?;
    validate(domain, &left.values)?;
    validate(domain, &right.values)?;
    let report = ComparisonPlan::new(domain, domain)?.compare(
        left.values.each_ref().map(Vec::as_slice),
        right.values.each_ref().map(Vec::as_slice),
    )?;
    let fine = field_norms(domain, &right.values)?;
    let normalized = ratios(report.full, fine)?;
    let mut projected = projected_difference(domain, &left.values, &right.values)?;
    let projected_norms = field_norms(domain, &projected)?;
    apply_stokes_response(domain, &mut projected)?;
    let stokes_response_norms = field_norms(domain, &projected)?;
    println!(
        "terminal=complete analyzer_source={} artifact_source={} retained={N} left_sampled={} right_sampled={} left_coefficient_sha256={} right_coefficient_sha256={} raw_difference={report:?} fine_force_norms={fine:?} raw_full_over_fine=[{:.17e},{:.17e},{:.17e},{:.17e}] leray_projected_difference={projected_norms:?} stokes_response_time=0.00390625 stokes_response_norms={stokes_response_norms:?} stokes_formula=(1-exp(-|k|^2*T))/|k|^2_with_zero_mode_T note=same-retained-layout-so-common-equals-full-and-newly-resolved-is-zero;divergence-is-a-comparison-channel-not-a-validity-zero-target force_error_qualification=false nonlinear_error_bound=false accepted_pde_windows=0",
        analyzer_source(),
        env!("RUN_SOURCE"),
        left.sampled,
        right.sampled,
        fingerprint(&left.values),
        fingerprint(&right.values),
        normalized[0],
        normalized[1],
        normalized[2],
        normalized[3],
    );
    Ok(())
}

fn analyzer_source() -> &'static str {
    option_env!("ANALYZER_SOURCE").unwrap_or(env!("RUN_SOURCE"))
}

fn field_norms(domain: Domain, values: &[Vec<Complex64>; 3]) -> Result<Norms, SolverError> {
    let zero = output(domain)?;
    Ok(ComparisonPlan::new(domain, domain)?
        .compare(
            zero.each_ref().map(Vec::as_slice),
            values.each_ref().map(Vec::as_slice),
        )?
        .full)
}

fn ratios(difference: Norms, fine: Norms) -> Result<[f64; 4], SolverError> {
    let values = [
        difference.l2 / fine.l2,
        difference.h1 / fine.h1,
        difference.vorticity_l2 / fine.vorticity_l2,
        difference.divergence_l2 / fine.divergence_l2,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(values)
}

fn projected_difference(
    domain: Domain,
    left: &[Vec<Complex64>; 3],
    right: &[Vec<Complex64>; 3],
) -> Result<[Vec<Complex64>; 3], SolverError> {
    let mut result = output(domain)?;
    let layout = domain.layout();
    for index in 0..layout.half_len() {
        let position = layout.position(index)?;
        let k = modal::wavevector(domain, layout.mode(position)?)?;
        let raw = std::array::from_fn(|axis| right[axis][index] - left[axis][index]);
        let projected = modal::project(k, raw)?;
        for axis in 0..3 {
            result[axis][index] = projected[axis];
        }
    }
    validate(domain, &result)?;
    Ok(result)
}

fn apply_stokes_response(
    domain: Domain,
    projected: &mut [Vec<Complex64>; 3],
) -> Result<(), SolverError> {
    const TIME: f64 = 4096.0 / 1_048_576.0;
    let layout = domain.layout();
    for index in 0..layout.half_len() {
        let position = layout.position(index)?;
        let k = modal::wavevector(domain, layout.mode(position)?)?;
        let squared = k.iter().map(|value| value * value).sum::<f64>();
        let factor = if squared == 0.0 {
            TIME
        } else {
            -(-squared * TIME).exp_m1() / squared
        };
        if !factor.is_finite() {
            return Err(SolverError::InvalidSpectrum);
        }
        for component in projected.iter_mut() {
            component[index] *= factor;
        }
    }
    validate(domain, projected)
}

fn output(domain: Domain) -> Result<[Vec<Complex64>; 3], SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(domain.layout().half_len())
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(domain.layout().half_len(), Complex64::new(0.0, 0.0));
    Ok([values.clone(), values.clone(), values])
}

fn validate(domain: Domain, values: &[Vec<Complex64>; 3]) -> Result<(), SolverError> {
    for component in values {
        if component.iter().any(|value| !value.is_finite()) {
            return Err(SolverError::InvalidSpectrum);
        }
        validate_spectrum(domain.layout(), component, 1e-12)?;
    }
    Ok(())
}

fn fingerprint(values: &[Vec<Complex64>; 3]) -> String {
    let mut hash = Sha256::new();
    for value in values.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn refuse_existing(path: &str) -> Result<(), SolverError> {
    let target = Path::new(path);
    let partial = partial_path(target);
    if target.exists() || partial.exists() {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

fn partial_path(target: &Path) -> PathBuf {
    let mut value = target.as_os_str().to_os_string();
    value.push(".partial");
    PathBuf::from(value)
}

fn write_artifact(
    path: &str,
    sampled: usize,
    values: &[Vec<Complex64>; 3],
) -> Result<(String, u64), SolverError> {
    let target = Path::new(path);
    let partial = partial_path(target);
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&partial)
        .map_err(|_| SolverError::InvalidPayload)?;
    let mut writer = BufWriter::with_capacity(ARTIFACT_BUFFER_BYTES, file);
    let mut hash = Sha256::new();
    write_hashed(&mut writer, &mut hash, &MAGIC)?;
    write_hashed(&mut writer, &mut hash, &1_u64.to_le_bytes())?;
    write_hashed(&mut writer, &mut hash, env!("RUN_SOURCE").as_bytes())?;
    write_hashed(&mut writer, &mut hash, CASE_SHA256.as_bytes())?;
    for value in [N as u64; 3] {
        write_hashed(&mut writer, &mut hash, &value.to_le_bytes())?;
    }
    for value in [sampled as u64; 3] {
        write_hashed(&mut writer, &mut hash, &value.to_le_bytes())?;
    }
    write_hashed(&mut writer, &mut hash, &(WORKERS as u64).to_le_bytes())?;
    write_hashed(&mut writer, &mut hash, &(-20_i64).to_le_bytes())?;
    for value in [8192_u128, 4096, 4096] {
        write_hashed(&mut writer, &mut hash, &value.to_le_bytes())?;
    }
    write_hashed(
        &mut writer,
        &mut hash,
        &(values[0].len() as u64).to_le_bytes(),
    )?;
    for value in values.iter().flatten() {
        write_hashed(&mut writer, &mut hash, &value.re.to_bits().to_le_bytes())?;
        write_hashed(&mut writer, &mut hash, &value.im.to_bits().to_le_bytes())?;
    }
    writer.flush().map_err(|_| SolverError::InvalidPayload)?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|_| SolverError::InvalidPayload)?;
    let bytes = writer
        .get_ref()
        .metadata()
        .map_err(|_| SolverError::InvalidPayload)?
        .len();
    drop(writer);
    fs::rename(&partial, target).map_err(|_| SolverError::InvalidPayload)?;
    Ok((format!("{:x}", hash.finalize()), bytes))
}

fn write_hashed(
    writer: &mut impl Write,
    hash: &mut Sha256,
    bytes: &[u8],
) -> Result<(), SolverError> {
    writer
        .write_all(bytes)
        .map_err(|_| SolverError::InvalidPayload)?;
    hash.update(bytes);
    Ok(())
}

fn read_artifact(path: &str) -> Result<Spectrum, SolverError> {
    let file = File::open(path).map_err(|_| SolverError::InvalidPayload)?;
    let mut reader = BufReader::with_capacity(ARTIFACT_BUFFER_BYTES, file);
    if read_array::<16>(&mut reader)? != MAGIC || read_u64(&mut reader)? != 1 {
        return Err(SolverError::InvalidPayload);
    }
    if read_array::<SOURCE_LENGTH>(&mut reader)? != *env!("RUN_SOURCE").as_bytes()
        || read_array::<CASE_LENGTH>(&mut reader)? != *CASE_SHA256.as_bytes()
    {
        return Err(SolverError::InvalidPayload);
    }
    let retained = [
        read_u64(&mut reader)?,
        read_u64(&mut reader)?,
        read_u64(&mut reader)?,
    ];
    let sampled = [
        read_u64(&mut reader)?,
        read_u64(&mut reader)?,
        read_u64(&mut reader)?,
    ];
    let workers = read_u64(&mut reader)?;
    let exponent = i64::from_le_bytes(read_array(&mut reader)?);
    let target = u128::from_le_bytes(read_array(&mut reader)?);
    let elapsed = u128::from_le_bytes(read_array(&mut reader)?);
    let remaining = u128::from_le_bytes(read_array(&mut reader)?);
    let length = read_u64(&mut reader)? as usize;
    let layout = Layout::new([N; 3])?;
    if retained != [N as u64; 3]
        || sampled[0] != sampled[1]
        || sampled[1] != sampled[2]
        || ![384, 512, 576, 768].contains(&(sampled[0] as usize))
        || workers != WORKERS as u64
        || exponent != -20
        || [target, elapsed, remaining] != [8192, 4096, 4096]
        || length != layout.half_len()
    {
        return Err(SolverError::InvalidPayload);
    }
    let mut values = output(Domain::new([N; 3], [1.0; 3], 1.0)?)?;
    for value in values.iter_mut().flatten() {
        value.re = f64::from_bits(read_u64(&mut reader)?);
        value.im = f64::from_bits(read_u64(&mut reader)?);
    }
    let mut trailing = [0_u8; 1];
    if reader
        .read(&mut trailing)
        .map_err(|_| SolverError::InvalidPayload)?
        != 0
    {
        return Err(SolverError::InvalidPayload);
    }
    Ok(Spectrum {
        sampled: sampled[0] as usize,
        values,
    })
}

fn read_u64(reader: &mut impl Read) -> Result<u64, SolverError> {
    Ok(u64::from_le_bytes(read_array(reader)?))
}

fn read_array<const N: usize>(reader: &mut impl Read) -> Result<[u8; N], SolverError> {
    let mut bytes = [0_u8; N];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| SolverError::InvalidPayload)?;
    Ok(bytes)
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, SolverError> {
    value.parse().map_err(|_| SolverError::InvalidPayload)
}
