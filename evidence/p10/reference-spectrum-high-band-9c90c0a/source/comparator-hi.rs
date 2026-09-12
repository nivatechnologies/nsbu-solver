use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{archive, Plan, Settings},
};
use nsbu_solver::{
    domain::{validate_spectrum, Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    spectral::modal,
    Complex64,
};
use sha2::{Digest, Sha256};
use std::{env, fs};

const CAP: usize = 1024 * 1024 * 1024;
const N: usize = 12;
const COMPONENTS: usize = 3;
const WORD_BYTES: usize = 16;
const FIXED_SCRATCH: usize = 4096;

fn main() {
    if let Err(error) = execute() {
        eprintln!("terminal=error error={error}");
        std::process::exit(2);
    }
}

fn execute() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("extract") if args.len() == 7 => extract(&args[2..]),
        Some("compare") if args.len() == 13 => compare(&args[2..]),
        _ => Err("usage: extract ARCHIVE OUTPUT CLOCK FORCE_M WORKERS | compare LEFT RIGHT CLOCK LEFT_N RIGHT_N LEFT_M RIGHT_M LEFT_WORKERS RIGHT_WORKERS LEFT_SHA256 RIGHT_SHA256".into()),
    }
}

fn extract(args: &[String]) -> Result<(), String> {
    let clock = number(&args[2])?;
    let force = number(&args[3])?;
    let workers = number(&args[4])?;
    let plan = plan(force, workers)?;
    let maximum = archive::maximum_encoded_len(plan).map_err(debug)?;
    let snapshot = snapshot_bytes(N)?;
    let reservation = archive::read_reservation(plan, 256)
        .map_err(debug)?
        .checked_add(maximum)
        .and_then(|n| n.checked_add(snapshot))
        .and_then(|n| n.checked_add(FIXED_SCRATCH))
        .ok_or("reservation overflow")?;
    println!(
        "mode=extract source={} cap={} reservation={} archive_maximum={} snapshot_bytes={}",
        env!("FORENSICS_SOURCE"),
        CAP,
        reservation,
        maximum,
        snapshot_bytes(N)?
    );
    if reservation > CAP {
        return Err("extract reservation exceeds cap".into());
    }
    let bytes = fs::read(&args[0]).map_err(debug)?;
    let imported = archive::read(&bytes, plan, maximum, CAP).map_err(debug)?;
    if imported.state().clock().elapsed() != clock as u128 {
        return Err("archive clock mismatch".into());
    }
    let output = encode(imported.state())?;
    fs::write(&args[1], &output).map_err(debug)?;
    println!(
        "snapshot={} clock={} force_m={} workers={} bytes={} sha256={} origin={:?}",
        args[1],
        clock,
        force,
        workers,
        output.len(),
        digest(&output),
        imported.origin()
    );
    println!("terminal=extract-complete");
    Ok(())
}

fn compare(args: &[String]) -> Result<(), String> {
    let profile = ComparisonProfile::from_args(args)?;
    let (reservation, visits, left_snapshot, right_snapshot) = comparison_resources(profile)?;
    println!("mode=compare source={} cap={} reservation={} left_snapshot_bytes={} right_snapshot_bytes={} coefficient_visits={} clock={} left_n={} right_n={} left_m={} right_m={} left_workers={} right_workers={}", env!("FORENSICS_SOURCE"), CAP, reservation, left_snapshot, right_snapshot, visits, profile.clock, profile.left_n, profile.right_n, profile.left_m, profile.right_m, profile.left_workers, profile.right_workers);
    if reservation > CAP {
        return Err("comparison reservation exceeds cap".into());
    }
    let (left_bytes, left) = load_snapshot(&args[0], &args[9], profile.left_n)?;
    let (right_bytes, right) = load_snapshot(&args[1], &args[10], profile.right_n)?;
    let coarse = Domain::new([profile.left_n; 3], [1.0; 3], 1.0).map_err(debug)?;
    let fine = Domain::new([profile.right_n; 3], [1.0; 3], 1.0).map_err(debug)?;
    validate(coarse, &left)?;
    validate(fine, &right)?;
    let [full, common, newly] = norms(coarse, fine, &left, &right)?;
    let mean_error: [f64; 3] = std::array::from_fn(|axis| right[axis][0].re - left[axis][0].re);
    println!("left_path={} left_sha256={} right_path={} right_sha256={} full={:?} common={:?} newly_resolved={:?} mean_error={:?} metadata_binding=externally_declared_and_hash_bound", args[0], digest(&left_bytes), args[1], digest(&right_bytes), full, common, newly, mean_error);
    println!("terminal=compare-complete");
    Ok(())
}

#[derive(Clone, Copy)]
struct ComparisonProfile {
    clock: usize,
    left_n: usize,
    right_n: usize,
    left_m: usize,
    right_m: usize,
    left_workers: usize,
    right_workers: usize,
}
impl ComparisonProfile {
    fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            clock: number(&args[2])?,
            left_n: number(&args[3])?,
            right_n: number(&args[4])?,
            left_m: number(&args[5])?,
            right_m: number(&args[6])?,
            left_workers: number(&args[7])?,
            right_workers: number(&args[8])?,
        })
    }
}

fn comparison_resources(p: ComparisonProfile) -> Result<(usize, usize, usize, usize), String> {
    let left_layout = Layout::new([p.left_n; 3]).map_err(debug)?;
    let right_layout = Layout::new([p.right_n; 3]).map_err(debug)?;
    let left_bytes = snapshot_bytes(p.left_n)?;
    let right_bytes = snapshot_bytes(p.right_n)?;
    let pair_bytes = left_bytes
        .checked_add(right_bytes)
        .ok_or("reservation overflow")?;
    let reservation = pair_bytes
        .checked_mul(2)
        .and_then(|n| n.checked_add(FIXED_SCRATCH))
        .ok_or("reservation overflow")?;
    let visits = left_layout
        .half_len()
        .checked_add(right_layout.half_len())
        .and_then(|n| n.checked_mul(3))
        .and_then(|n| n.checked_add(right_layout.half_len()))
        .ok_or("work overflow")?;
    Ok((reservation, visits, left_bytes, right_bytes))
}

fn load_snapshot(
    path: &str,
    expected: &str,
    n: usize,
) -> Result<(Vec<u8>, [Vec<Complex64>; 3]), String> {
    let bytes = fs::read(path).map_err(debug)?;
    if digest(&bytes) != expected {
        return Err("snapshot hash mismatch".into());
    }
    let values = decode(&bytes, n)?;
    Ok((bytes, values))
}

fn validate(domain: Domain, values: &[Vec<Complex64>; 3]) -> Result<(), String> {
    for component in values {
        validate_spectrum(domain.layout(), component, 1e-12).map_err(debug)?;
    }
    Ok(())
}

fn norms(
    coarse: Domain,
    fine: Domain,
    left: &[Vec<Complex64>; 3],
    right: &[Vec<Complex64>; 3],
) -> Result<[Norms; 3], String> {
    if coarse.lengths() != fine.lengths()
        || coarse.viscosity() != fine.viscosity()
        || coarse
            .layout()
            .dimensions()
            .into_iter()
            .zip(fine.layout().dimensions())
            .any(|(a, b)| a > b)
    {
        return Err("incompatible domains".into());
    }
    let mut common = NormSums::default();
    let mut newly = NormSums::default();
    for (index, _) in right[0].iter().enumerate() {
        let position = fine.layout().position(index).map_err(debug)?;
        if fine.layout().is_nyquist(position).map_err(debug)? {
            continue;
        }
        let mode = fine.layout().mode(position).map_err(debug)?;
        let earlier = coarse.layout().locate(mode).ok().map(|(i, _)| i);
        let difference: [Complex64; 3] = std::array::from_fn(|axis| {
            right[axis][index] - earlier.map_or(Complex64::new(0.0, 0.0), |i| left[axis][i])
        });
        let target = if earlier.is_some() {
            &mut common
        } else {
            &mut newly
        };
        target.push(
            modal::wavevector(fine, mode).map_err(debug)?,
            difference,
            fine.layout().weight(position).map_err(debug)?,
        )?;
    }
    let common = common.finish()?;
    let newly = newly.finish()?;
    Ok([common.orthogonal(newly)?, common, newly])
}

#[derive(Clone, Copy, Debug)]
struct Norms {
    l2: f64,
    h1: f64,
    vorticity_l2: f64,
    divergence_l2: f64,
}
impl Norms {
    fn orthogonal(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            l2: finite(self.l2.hypot(other.l2))?,
            h1: finite(self.h1.hypot(other.h1))?,
            vorticity_l2: finite(self.vorticity_l2.hypot(other.vorticity_l2))?,
            divergence_l2: finite(self.divergence_l2.hypot(other.divergence_l2))?,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct NormSums {
    l2: Squares,
    h1: Squares,
    curl: Squares,
    divergence: Squares,
}
impl NormSums {
    fn push(&mut self, k: [f64; 3], u: [Complex64; 3], weight: f64) -> Result<(), String> {
        for value in u {
            self.l2.complex(value, weight)?;
            self.h1.complex(value, weight)?;
            for frequency in k {
                self.h1.complex(value * frequency, weight)?;
            }
        }
        for (next, last) in [(1, 2), (2, 0), (0, 1)] {
            self.curl
                .complex(k[next] * u[last] - k[last] * u[next], weight)?;
        }
        self.divergence
            .complex(k[0] * u[0] + k[1] * u[1] + k[2] * u[2], weight)
    }
    fn finish(self) -> Result<Norms, String> {
        Ok(Norms {
            l2: self.l2.norm()?,
            h1: self.h1.norm()?,
            vorticity_l2: self.curl.norm()?,
            divergence_l2: self.divergence.norm()?,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct Squares {
    scale: f64,
    sum: f64,
}
impl Squares {
    fn complex(&mut self, value: Complex64, weight: f64) -> Result<(), String> {
        self.push(value.re, weight)?;
        self.push(value.im, weight)
    }
    fn push(&mut self, value: f64, weight: f64) -> Result<(), String> {
        let value = value.abs();
        if !value.is_finite() {
            return Err("nonfinite difference".into());
        }
        if value != 0.0 {
            let scale = self.scale.max(value);
            self.sum = self.sum * (self.scale / scale).powi(2) + weight * (value / scale).powi(2);
            self.scale = scale;
        }
        Ok(())
    }
    fn norm(self) -> Result<f64, String> {
        let value = self.scale * self.sum.sqrt();
        if value.is_finite() {
            Ok(value)
        } else {
            Err("nonfinite norm".into())
        }
    }
}

fn plan(force: usize, workers: usize) -> Result<Plan, String> {
    Plan::from_rest(
        Settings {
            domain: Domain::new([N; 3], [1.0; 3], 1.0).map_err(debug)?,
            force: ForceSettings {
                samples: Layout::new([force; 3]).map_err(debug)?,
                workers,
            },
            initial_clock: TickClock::from_rest(-20, 8192).map_err(debug)?,
            configuration: Configuration {
                limits: RunLimits {
                    endpoint: 4096,
                    step_ticks: 16,
                    maximum_attempts: 256,
                },
                method: Method::CoxMatthews,
                tolerances: Tolerances {
                    absolute: [1e-5, 1e-4],
                    relative: [0.0; 2],
                },
            },
            advective_limit: 0.3,
        },
        CAP,
    )
    .map_err(debug)
}

fn snapshot_bytes(n: usize) -> Result<usize, String> {
    Layout::new([n; 3])
        .map_err(debug)?
        .half_len()
        .checked_mul(COMPONENTS)
        .and_then(|n| n.checked_mul(WORD_BYTES))
        .ok_or("snapshot overflow".into())
}
fn encode(state: &nsbu_solver::domain::SpectralState) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.try_reserve_exact(snapshot_bytes(N)?).map_err(debug)?;
    for axis in 0..3 {
        for z in state.component(axis).map_err(debug)? {
            out.extend_from_slice(&z.re.to_bits().to_le_bytes());
            out.extend_from_slice(&z.im.to_bits().to_le_bytes());
        }
    }
    Ok(out)
}
fn decode(bytes: &[u8], layout_n: usize) -> Result<[Vec<Complex64>; 3], String> {
    if bytes.len() != snapshot_bytes(layout_n)? {
        return Err("snapshot byte length mismatch".into());
    }
    let n = bytes.len() / (COMPONENTS * WORD_BYTES);
    Ok(std::array::from_fn(|axis| {
        (0..n)
            .map(|i| {
                let p = (axis * n + i) * 16;
                Complex64::new(
                    f64::from_bits(u64::from_le_bytes(bytes[p..p + 8].try_into().unwrap())),
                    f64::from_bits(u64::from_le_bytes(bytes[p + 8..p + 16].try_into().unwrap())),
                )
            })
            .collect()
    }))
}
fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}
fn number(value: &str) -> Result<usize, String> {
    value.parse().map_err(debug)
}
fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn finite(value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("nonfinite norm".into())
    }
}
