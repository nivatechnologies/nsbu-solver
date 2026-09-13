//! Force-only clock-1112 sampling discriminant. No trajectory or acceptance state is created.
use nsbu_benchmarks::{provider::parallel_reduced::ParallelReducedV2Force, CASE_SHA256};
use nsbu_solver::{
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
const N: usize = 384;
const WORKERS: usize = 32;
const CLOCK: u128 = 1112;
const TARGET: u128 = 8192;
const EXPONENT: i32 = -20;
const ARTIFACT_BUFFER_BYTES: usize = 1024 * 1024;
const METADATA_BYTES: usize = 64 * 1024;
const SOURCE_LENGTH: usize = 40;
const CASE_LENGTH: usize = 64;
const MAGIC: [u8; 16] = *b"NSBUFORCEv2\0\0\0\0\0";
const SAMPLE_GRIDS: [usize; 4] = [384, 512, 768, 1024];
const SHELLS: [(&str, usize); 7] = [
    ("inside_n48", 24),
    ("n48_to_n64", 32),
    ("n64_to_n96", 48),
    ("n96_to_n128", 64),
    ("n128_to_n192", 96),
    ("n192_to_n256", 128),
    ("n256_to_n384", 192),
];

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
    if !SAMPLE_GRIDS.contains(&m) || env!("RUN_SOURCE").len() != SOURCE_LENGTH {
        return Err(SolverError::InvalidPayload);
    }
    let domain = domain()?;
    let samples = Layout::new([m; 3])?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let limits =
        ParallelReducedV2Force::preflight_with_fft_backend(domain, samples, WORKERS, backend)?;
    let output_bytes = domain
        .layout()
        .half_len()
        .checked_mul(3 * size_of::<Complex64>())
        .ok_or(SolverError::SizeOverflow)?;
    let peak = catalog_bytes
        .checked_add(limits.storage_bytes)
        .and_then(|value| value.checked_add(2 * output_bytes))
        .and_then(|value| value.checked_add(ARTIFACT_BUFFER_BYTES + METADATA_BYTES))
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} case_sha256={CASE_SHA256} purpose=clock1112-force-alias-discriminant retained={N} sampled={m} workers={WORKERS} backend={backend:?} exponent={EXPONENT} target={TARGET} elapsed={CLOCK} remaining={} catalog_bytes={catalog_bytes} force={limits:?} output_bytes={output_bytes} peak_with_previous_spectrum_bytes={peak} cap={CAP} output={output_path} trajectory_evolved=false accepted_pde_windows=0",
        env!("RUN_SOURCE"), TARGET - CLOCK,
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
    let force_started = Instant::now();
    let work = provider.evaluate(clock()?, limits, values.each_mut().map(Vec::as_mut_slice))?;
    let force_seconds = force_started.elapsed().as_secs_f64();
    validate(domain, &values)?;
    let coefficient_sha256 = fingerprint(&values);
    let (artifact_sha256, artifact_bytes) = write_artifact(output_path, m, &values)?;
    println!(
        "terminal=complete source={} retained={N} sampled={m} clock={CLOCK} catalog_seconds={catalog_seconds:.9} construction_seconds={construction_seconds:.9} force_seconds={force_seconds:.9} work={work:?} root_iterations={} provider_identity={:?} coefficient_sha256={coefficient_sha256} artifact_sha256={artifact_sha256} artifact_bytes={artifact_bytes} force_sufficiency=false accepted_pde_windows=0",
        env!("RUN_SOURCE"), provider.last_root_iterations(), provider.identity(),
    );
    Ok(())
}

fn compare(left_path: &str, right_path: &str) -> Result<(), SolverError> {
    let left = read_artifact(left_path)?;
    let right = read_artifact(right_path)?;
    if left.sampled >= right.sampled {
        return Err(SolverError::InvalidPayload);
    }
    let domain = domain()?;
    validate(domain, &left.values)?;
    validate(domain, &right.values)?;
    let mut raw = [ShellSums::default(); SHELLS.len()];
    let mut projected = [ShellSums::default(); SHELLS.len()];
    let mut stokes = [ShellSums::default(); SHELLS.len()];
    let layout = domain.layout();
    for index in 0..layout.half_len() {
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            continue;
        }
        let mode = layout.mode(position)?;
        let shell = shell_index(mode).ok_or(SolverError::InvalidSpectrum)?;
        let k = modal::wavevector(domain, mode)?;
        let weight = layout.weight(position)?;
        let difference =
            std::array::from_fn(|axis| right.values[axis][index] - left.values[axis][index]);
        let solenoidal = modal::project(k, difference)?;
        raw[shell].push(k, difference, weight)?;
        projected[shell].push(k, solenoidal, weight)?;
        let factor = stokes_factor(k, CLOCK as f64 * 2_f64.powi(EXPONENT))?;
        stokes[shell].push(k, solenoidal.map(|value| value * factor), weight)?;
    }
    let raw = finish_shells(raw)?;
    let projected = finish_shells(projected)?;
    let stokes = finish_shells(stokes)?;
    println!(
        "terminal=complete analyzer_source={} artifact_source={} retained={N} clock={CLOCK} left_sampled={} right_sampled={} left_coefficient_sha256={} right_coefficient_sha256={} note=contracting-retained-pairs-support-improved-sampled-force-convergence-but-do-not-prove-a-continuum-tail force_sufficiency=false accepted_pde_windows=0",
        analyzer_source(), env!("RUN_SOURCE"), left.sampled, right.sampled,
        fingerprint(&left.values), fingerprint(&right.values),
    );
    for (index, (name, _)) in SHELLS.into_iter().enumerate() {
        println!(
            "shell={name} raw={:?} leray_projected={:?} stokes_response={:?}",
            raw[index], projected[index], stokes[index]
        );
    }
    println!(
        "full raw={:?} leray_projected={:?} stokes_response={:?}",
        combine(&raw)?,
        combine(&projected)?,
        combine(&stokes)?,
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct ShellSums {
    l2: f64,
    h1: f64,
    curl: f64,
    divergence: f64,
}

impl ShellSums {
    fn push(&mut self, k: [f64; 3], u: [Complex64; 3], weight: f64) -> Result<(), SolverError> {
        let magnitude = |value: Complex64| value.norm_sqr();
        let l2 = u.into_iter().map(magnitude).sum::<f64>();
        let k2 = k.into_iter().map(|value| value * value).sum::<f64>();
        let curl = [
            k[1] * u[2] - k[2] * u[1],
            k[2] * u[0] - k[0] * u[2],
            k[0] * u[1] - k[1] * u[0],
        ];
        let divergence = k[0] * u[0] + k[1] * u[1] + k[2] * u[2];
        for value in [
            weight * l2,
            weight * (1.0 + k2) * l2,
            weight * curl.into_iter().map(magnitude).sum::<f64>(),
            weight * magnitude(divergence),
        ] {
            if !value.is_finite() {
                return Err(SolverError::InvalidSpectrum);
            }
        }
        self.l2 += weight * l2;
        self.h1 += weight * (1.0 + k2) * l2;
        self.curl += weight * curl.into_iter().map(magnitude).sum::<f64>();
        self.divergence += weight * magnitude(divergence);
        Ok(())
    }

    fn finish(self) -> Result<Norms, SolverError> {
        let values = [self.l2, self.h1, self.curl, self.divergence].map(f64::sqrt);
        if values.iter().any(|value| !value.is_finite()) {
            return Err(SolverError::InvalidSpectrum);
        }
        Ok(Norms {
            l2: values[0],
            h1: values[1],
            vorticity_l2: values[2],
            divergence_l2: values[3],
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Norms {
    l2: f64,
    h1: f64,
    vorticity_l2: f64,
    divergence_l2: f64,
}

fn finish_shells(sums: [ShellSums; SHELLS.len()]) -> Result<[Norms; SHELLS.len()], SolverError> {
    let mut result = [Norms {
        l2: 0.0,
        h1: 0.0,
        vorticity_l2: 0.0,
        divergence_l2: 0.0,
    }; SHELLS.len()];
    for (target, source) in result.iter_mut().zip(sums) {
        *target = source.finish()?;
    }
    Ok(result)
}

fn combine(shells: &[Norms; SHELLS.len()]) -> Result<Norms, SolverError> {
    let norm = |select: fn(&Norms) -> f64| {
        shells
            .iter()
            .map(|item| select(item).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let result = Norms {
        l2: norm(|item| item.l2),
        h1: norm(|item| item.h1),
        vorticity_l2: norm(|item| item.vorticity_l2),
        divergence_l2: norm(|item| item.divergence_l2),
    };
    if [
        result.l2,
        result.h1,
        result.vorticity_l2,
        result.divergence_l2,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(result)
}

fn shell_index(mode: [isize; 3]) -> Option<usize> {
    let maximum = mode.into_iter().map(isize::unsigned_abs).max()?;
    SHELLS.iter().position(|(_, cutoff)| maximum < *cutoff)
}

fn stokes_factor(k: [f64; 3], time: f64) -> Result<f64, SolverError> {
    let squared = k.into_iter().map(|value| value * value).sum::<f64>();
    let factor = if squared == 0.0 {
        time
    } else {
        -(-squared * time).exp_m1() / squared
    };
    if !factor.is_finite() {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(factor)
}

fn domain() -> Result<Domain, SolverError> {
    Domain::new([N; 3], [1.0; 3], 1.0)
}

fn clock() -> Result<TickClock, SolverError> {
    TickClock::restore(EXPONENT, TARGET, CLOCK, TARGET - CLOCK)
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

fn analyzer_source() -> &'static str {
    option_env!("ANALYZER_SOURCE").unwrap_or(env!("RUN_SOURCE"))
}

fn refuse_existing(path: &str) -> Result<(), SolverError> {
    let target = Path::new(path);
    if target.exists() || partial_path(target).exists() {
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
    for bytes in [
        MAGIC.as_slice(),
        2_u64.to_le_bytes().as_slice(),
        env!("RUN_SOURCE").as_bytes(),
        CASE_SHA256.as_bytes(),
    ] {
        write_hashed(&mut writer, &mut hash, bytes)?;
    }
    for value in [N as u64, sampled as u64, WORKERS as u64] {
        write_hashed(&mut writer, &mut hash, &value.to_le_bytes())?;
    }
    write_hashed(&mut writer, &mut hash, &(EXPONENT as i64).to_le_bytes())?;
    for value in [TARGET, CLOCK, TARGET - CLOCK] {
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
    if read_array::<16>(&mut reader)? != MAGIC
        || read_u64(&mut reader)? != 2
        || read_array::<SOURCE_LENGTH>(&mut reader)? != *env!("RUN_SOURCE").as_bytes()
        || read_array::<CASE_LENGTH>(&mut reader)? != *CASE_SHA256.as_bytes()
    {
        return Err(SolverError::InvalidPayload);
    }
    let retained = read_u64(&mut reader)? as usize;
    let sampled = read_u64(&mut reader)? as usize;
    let workers = read_u64(&mut reader)? as usize;
    let exponent = i64::from_le_bytes(read_array(&mut reader)?);
    let target = u128::from_le_bytes(read_array(&mut reader)?);
    let elapsed = u128::from_le_bytes(read_array(&mut reader)?);
    let remaining = u128::from_le_bytes(read_array(&mut reader)?);
    let length = read_u64(&mut reader)? as usize;
    let layout = Layout::new([N; 3])?;
    if retained != N
        || !SAMPLE_GRIDS.contains(&sampled)
        || workers != WORKERS
        || exponent != EXPONENT as i64
        || [target, elapsed, remaining] != [TARGET, CLOCK, TARGET - CLOCK]
        || length != layout.half_len()
    {
        return Err(SolverError::InvalidPayload);
    }
    let mut values = output(domain()?)?;
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
    Ok(Spectrum { sampled, values })
}

fn read_u64(reader: &mut impl Read) -> Result<u64, SolverError> {
    Ok(u64::from_le_bytes(read_array(reader)?))
}

fn read_array<const SIZE: usize>(reader: &mut impl Read) -> Result<[u8; SIZE], SolverError> {
    let mut bytes = [0_u8; SIZE];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| SolverError::InvalidPayload)?;
    Ok(bytes)
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, SolverError> {
    value.parse().map_err(|_| SolverError::InvalidPayload)
}

#[cfg(test)]
mod tests;
