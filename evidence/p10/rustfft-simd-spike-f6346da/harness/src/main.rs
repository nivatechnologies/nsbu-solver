use nsbu_solver::{domain::Layout, spectral::FftPlan, Complex64, SolverError};
use rustfft::{Fft, FftPlannerAvx};
use sha2::{Digest, Sha256};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    process::ExitCode,
    sync::Arc,
    time::{Duration, Instant},
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const SOURCE: &str = "f6346da8085365fad84874db08aa1213c75df331";
const FORMATTING_ALLOWANCE: usize = 64 * 1024;

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal=error detail={error}");
            ExitCode::from(1)
        }
    }
}

fn execute() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let dimensions = match arguments.next().as_deref() {
        Some("--validate") => {
            validate_direct_dft()?;
            println!("terminal=validation-complete");
            return Ok(());
        }
        Some(value) => vec![parse(value)?],
        None => vec![192, 288, 384],
    };
    if arguments.next().is_some() {
        return Err("usage: p10-rustfft-simd-spike [N|--validate]".into());
    }
    println!(
        "identity source={SOURCE} rustfft=6.4.1 simd_planner=avx runtime_avx={} runtime_fma={} runtime_avx2={} rustc={} target={} axis_order=z,x,y normalization=forward_divide_real_len strict_half_layout=true",
        std::is_x86_feature_detected!("avx"),
        std::is_x86_feature_detected!("fma"),
        std::is_x86_feature_detected!("avx2"),
        rustc_version(),
        std::env::consts::ARCH,
    );
    for n in dimensions {
        profile(n)?;
    }
    println!("terminal=profile-complete");
    Ok(())
}

fn profile(n: usize) -> Result<(), String> {
    let layout = Layout::new([n; 3]).map_err(debug)?;
    let owned_reservation = FftPlan::reservation(layout).map_err(debug)?;
    let io_bytes = io_bytes(layout)?;
    let (mut owned, mut rust) = owners(layout, owned_reservation)?;
    let rust_accounted = rust.accounted_bytes(io_bytes)?;
    let joint = owned_reservation
        .checked_add(io_bytes)
        .and_then(|value| value.checked_add(rust_accounted))
        .and_then(|value| value.checked_add(FORMATTING_ALLOWANCE))
        .ok_or("joint reservation overflow")?;
    println!(
        "prototype_accounting n={n} production_preflight=false owned_fft_bytes={owned_reservation} io_bytes_each={io_bytes} rust_grid_bytes={} rust_row_bytes={} rust_scratch_complex={} rust_scratch_bytes={} rust_planner_allocations={} rust_planner_deallocations={} rust_planner_reallocations={} rust_planner_bytes_allocated={} rust_planner_bytes_deallocated={} rust_planner_measured_retained_bytes={} rust_owner_accounted_bytes={rust_accounted} formatting_allowance={FORMATTING_ALLOWANCE} joint_accounted_bytes={joint}",
        rust.grid.len() * 16,
        rust.row.len() * 16,
        rust.scratch.len(),
        rust.scratch.len() * 16,
        rust.planning.allocations,
        rust.planning.deallocations,
        rust.planning.reallocations,
        rust.planning.bytes_allocated,
        rust.planning.bytes_deallocated,
        rust.planner_retained(),
    );

    owned.reset(0);
    rust.reset(0);
    owned.cycle(1).map_err(debug)?;
    rust.cycle(1)?;

    let mut owned_times = [Duration::ZERO; 3];
    let mut rust_times = [Duration::ZERO; 3];
    for pair in 0..3 {
        owned.reset(pair + 1);
        rust.reset(pair + 1);
        if pair % 2 == 0 {
            owned_times[pair] = timed_owned(&mut owned, 2)?;
            rust_times[pair] = timed_rust(&mut rust, 2)?;
        } else {
            rust_times[pair] = timed_rust(&mut rust, 2)?;
            owned_times[pair] = timed_owned(&mut owned, 2)?;
        }
        let comparison = compare(&owned.spectrum, &rust.spectrum)?;
        let physical = compare_real(&owned.physical, &rust.physical)?;
        println!(
            "pair n={n} index={pair} owned_seconds={:.9} rustfft_seconds={:.9} speedup={:.6} spectral_max_scaled={:.17e} spectral_rms={:.17e} spectral_equal_words={}/{} physical_max_scaled={:.17e} physical_rms={:.17e} physical_equal_words={}/{} owned_spectral_sha256={} rustfft_spectral_sha256={} owned_physical_sha256={} rustfft_physical_sha256={}",
            owned_times[pair].as_secs_f64(),
            rust_times[pair].as_secs_f64(),
            owned_times[pair].as_secs_f64() / rust_times[pair].as_secs_f64(),
            comparison.maximum_scaled,
            comparison.rms,
            comparison.equal_words,
            comparison.words,
            physical.maximum_scaled,
            physical.rms,
            physical.equal_words,
            physical.words,
            hash_complex(&owned.spectrum),
            hash_complex(&rust.spectrum),
            hash_real(&owned.physical),
            hash_real(&rust.physical),
        );
    }
    owned_times.sort();
    rust_times.sort();
    let speedup = owned_times[1].as_secs_f64() / rust_times[1].as_secs_f64();

    owned.reset(17);
    rust.reset(17);
    let region = Region::new(GLOBAL);
    owned.cycle(1).map_err(debug)?;
    rust.cycle(1)?;
    let steady = region.change();
    if (
        steady.allocations,
        steady.deallocations,
        steady.reallocations,
    ) != (0, 0, 0)
    {
        return Err(format!("steady transform allocation at n={n}: {steady:?}"));
    }
    println!(
        "summary n={n} scalar_transforms_per_timing=4 owned_median_seconds={:.9} rustfft_median_seconds={:.9} median_speedup={speedup:.6} steady_allocations={} steady_deallocations={} steady_reallocations={} steady_bytes_allocated={} steady_bytes_deallocated={}",
        owned_times[1].as_secs_f64(),
        rust_times[1].as_secs_f64(),
        steady.allocations,
        steady.deallocations,
        steady.reallocations,
        steady.bytes_allocated,
        steady.bytes_deallocated,
    );
    Ok(())
}

fn timed_owned(owner: &mut Owned, repeats: usize) -> Result<Duration, String> {
    let started = Instant::now();
    owner.cycle(repeats).map_err(debug)?;
    Ok(started.elapsed())
}

fn timed_rust(owner: &mut Rust3d, repeats: usize) -> Result<Duration, String> {
    let started = Instant::now();
    owner.cycle(repeats)?;
    Ok(started.elapsed())
}

struct Owned {
    fft: FftPlan,
    work: nsbu_solver::spectral::FftWorkspace,
    physical: Vec<f64>,
    spectrum: Vec<Complex64>,
}

impl Owned {
    fn reset(&mut self, variant: usize) {
        fill_input(&mut self.physical, variant);
        self.spectrum.fill(Complex64::new(0.0, 0.0));
    }

    fn cycle(&mut self, repeats: usize) -> Result<(), SolverError> {
        for _ in 0..repeats {
            self.fft
                .forward(&self.physical, &mut self.spectrum, &mut self.work)?;
            self.fft
                .inverse(&self.spectrum, &mut self.physical, &mut self.work)?;
        }
        Ok(())
    }
}

struct Rust3d {
    layout: Layout,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
    planning: Stats,
    grid: Vec<Complex64>,
    row: Vec<Complex64>,
    scratch: Vec<Complex64>,
    physical: Vec<f64>,
    spectrum: Vec<Complex64>,
}

impl Rust3d {
    fn new(layout: Layout) -> Result<Self, String> {
        let [nx, ny, nz] = layout.dimensions();
        if nx != ny || ny != nz {
            return Err("prototype requires an isotropic layout".into());
        }
        let region = Region::new(GLOBAL);
        let mut planner =
            FftPlannerAvx::<f64>::new().map_err(|_| "RustFFT AVX/FMA planner unavailable")?;
        let forward = planner.plan_fft_forward(nz);
        let inverse = planner.plan_fft_inverse(nz);
        drop(planner);
        let planning = region.change();
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Ok(Self {
            layout,
            forward,
            inverse,
            planning,
            grid: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
            row: filled(nz, Complex64::new(0.0, 0.0))?,
            scratch: filled(scratch_len, Complex64::new(0.0, 0.0))?,
            physical: filled(layout.real_len(), 0.0)?,
            spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        })
    }

    fn reset(&mut self, variant: usize) {
        fill_input(&mut self.physical, variant);
        self.spectrum.fill(Complex64::new(0.0, 0.0));
    }

    fn accounted_bytes(&self, io_bytes: usize) -> Result<usize, String> {
        self.grid
            .len()
            .checked_mul(16)
            .and_then(|value| value.checked_add(self.row.len() * 16))
            .and_then(|value| value.checked_add(self.scratch.len() * 16))
            .and_then(|value| value.checked_add(io_bytes))
            .and_then(|value| value.checked_add(self.planner_retained()))
            .and_then(|value| value.checked_add(std::mem::size_of::<Self>() + 7 * 64))
            .ok_or_else(|| "RustFFT prototype accounting overflow".into())
    }

    fn planner_retained(&self) -> usize {
        self.planning
            .bytes_allocated
            .saturating_sub(self.planning.bytes_deallocated)
            .saturating_add_signed(self.planning.bytes_reallocated)
    }

    fn cycle(&mut self, repeats: usize) -> Result<(), String> {
        for _ in 0..repeats {
            self.forward()?;
            self.inverse()?;
        }
        Ok(())
    }

    fn forward(&mut self) -> Result<(), String> {
        finite_real(&self.physical)?;
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for row_index in 0..nx * ny {
            for k in 0..nz {
                self.row[k] = Complex64::new(self.physical[row_index * nz + k], 0.0);
            }
            self.forward
                .process_with_scratch(&mut self.row, &mut self.scratch);
            self.grid[row_index * half..(row_index + 1) * half].copy_from_slice(&self.row[..half]);
        }
        self.transverse(false);
        let scale = self.layout.real_len() as f64;
        for (value, transformed) in self.spectrum.iter_mut().zip(&self.grid) {
            *value = transformed / scale;
        }
        finite_complex(&self.spectrum)
    }

    fn inverse(&mut self) -> Result<(), String> {
        finite_complex(&self.spectrum)?;
        self.grid.copy_from_slice(&self.spectrum);
        self.transverse(true);
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for row_index in 0..nx * ny {
            self.row[..half].copy_from_slice(&self.grid[row_index * half..(row_index + 1) * half]);
            for k in half..nz {
                self.row[k] = self.row[nz - k].conj();
            }
            self.inverse
                .process_with_scratch(&mut self.row, &mut self.scratch);
            for k in 0..nz {
                self.physical[row_index * nz + k] = self.row[k].re;
            }
        }
        finite_real(&self.physical)
    }

    fn transverse(&mut self, inverse: bool) {
        let [nx, ny, nz] = self.layout.dimensions();
        let half = nz / 2 + 1;
        for axis in 0..2 {
            let (length, rows, stride) = if axis == 0 {
                (nx, ny, ny * half)
            } else {
                (ny, nx, half)
            };
            for row_index in 0..rows {
                let row_base = if axis == 0 {
                    row_index * half
                } else {
                    row_index * ny * half
                };
                for k in 0..half {
                    let base = row_base + k;
                    for j in 0..length {
                        self.row[j] = self.grid[base + j * stride];
                    }
                    if inverse {
                        self.inverse
                            .process_with_scratch(&mut self.row[..length], &mut self.scratch);
                    } else {
                        self.forward
                            .process_with_scratch(&mut self.row[..length], &mut self.scratch);
                    }
                    for j in 0..length {
                        self.grid[base + j * stride] = self.row[j];
                    }
                }
            }
        }
    }
}

fn owners(layout: Layout, cap: usize) -> Result<(Owned, Rust3d), String> {
    let (fft, work) = FftPlan::new(layout, cap).map_err(debug)?;
    let owned = Owned {
        fft,
        work,
        physical: filled(layout.real_len(), 0.0)?,
        spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
    };
    Ok((owned, Rust3d::new(layout)?))
}

fn validate_direct_dft() -> Result<(), String> {
    let layout = Layout::new([6; 3]).map_err(debug)?;
    let cap = FftPlan::reservation(layout).map_err(debug)?;
    let (mut owned, mut rust) = owners(layout, cap)?;
    owned.reset(23);
    rust.reset(23);
    let direct = direct_dft(layout, &owned.physical);
    owned
        .fft
        .forward(&owned.physical, &mut owned.spectrum, &mut owned.work)
        .map_err(debug)?;
    rust.forward()?;
    let owned_error = compare(&direct, &owned.spectrum)?;
    let rust_error = compare(&direct, &rust.spectrum)?;
    let cross = compare(&owned.spectrum, &rust.spectrum)?;
    if owned_error.maximum_scaled > 2e-14 || rust_error.maximum_scaled > 2e-14 {
        return Err(format!(
            "direct DFT mismatch: owned={} rustfft={}",
            owned_error.maximum_scaled, rust_error.maximum_scaled
        ));
    }
    owned
        .fft
        .inverse(&owned.spectrum, &mut owned.physical, &mut owned.work)
        .map_err(debug)?;
    rust.inverse()?;
    let roundtrip = compare_real(&owned.physical, &rust.physical)?;
    println!(
        "validation n=6 direct_owned_max_scaled={:.17e} direct_rustfft_max_scaled={:.17e} owned_rustfft_spectral_max_scaled={:.17e} owned_rustfft_spectral_equal_words={}/{} roundtrip_cross_max_scaled={:.17e}",
        owned_error.maximum_scaled,
        rust_error.maximum_scaled,
        cross.maximum_scaled,
        cross.equal_words,
        cross.words,
        roundtrip.maximum_scaled,
    );
    Ok(())
}

fn direct_dft(layout: Layout, input: &[f64]) -> Vec<Complex64> {
    let [nx, ny, nz] = layout.dimensions();
    let half = nz / 2 + 1;
    let scale = layout.real_len() as f64;
    let mut output = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for a in 0..nx {
        for b in 0..ny {
            for c in 0..half {
                let mut value = Complex64::new(0.0, 0.0);
                for i in 0..nx {
                    for j in 0..ny {
                        for k in 0..nz {
                            let angle = -std::f64::consts::TAU
                                * (a as f64 * i as f64 / nx as f64
                                    + b as f64 * j as f64 / ny as f64
                                    + c as f64 * k as f64 / nz as f64);
                            value += input[(i * ny + j) * nz + k]
                                * Complex64::new(angle.cos(), angle.sin());
                        }
                    }
                }
                output[(a * ny + b) * half + c] = value / scale;
            }
        }
    }
    output
}

#[derive(Clone, Copy)]
struct Difference {
    maximum_scaled: f64,
    rms: f64,
    equal_words: usize,
    words: usize,
}

fn compare(left: &[Complex64], right: &[Complex64]) -> Result<Difference, String> {
    if left.len() != right.len() {
        return Err("complex comparison length mismatch".into());
    }
    let mut maximum_scaled = 0.0_f64;
    let mut sum = 0.0;
    let mut equal_words = 0;
    for (&a, &b) in left.iter().zip(right) {
        maximum_scaled = maximum_scaled.max((a - b).norm() / (1.0 + a.norm()));
        sum += (a - b).norm_sqr();
        equal_words += usize::from(a.re.to_bits() == b.re.to_bits());
        equal_words += usize::from(a.im.to_bits() == b.im.to_bits());
    }
    Ok(Difference {
        maximum_scaled,
        rms: (sum / left.len() as f64).sqrt(),
        equal_words,
        words: 2 * left.len(),
    })
}

fn compare_real(left: &[f64], right: &[f64]) -> Result<Difference, String> {
    if left.len() != right.len() {
        return Err("real comparison length mismatch".into());
    }
    let mut maximum_scaled = 0.0_f64;
    let mut sum = 0.0;
    let mut equal_words = 0;
    for (&a, &b) in left.iter().zip(right) {
        maximum_scaled = maximum_scaled.max((a - b).abs() / (1.0 + a.abs()));
        sum += (a - b).powi(2);
        equal_words += usize::from(a.to_bits() == b.to_bits());
    }
    Ok(Difference {
        maximum_scaled,
        rms: (sum / left.len() as f64).sqrt(),
        equal_words,
        words: left.len(),
    })
}

fn fill_input(values: &mut [f64], variant: usize) {
    for (index, value) in values.iter_mut().enumerate() {
        let numerator = ((index.wrapping_mul(17) + variant * 13) % 257) as f64 - 128.0;
        *value = numerator / 257.0 + (variant as f64 + 1.0) / 4096.0;
    }
}

fn io_bytes(layout: Layout) -> Result<usize, String> {
    layout
        .real_len()
        .checked_mul(8)
        .and_then(|value| value.checked_add(layout.half_len() * 16))
        .ok_or_else(|| "I/O reservation overflow".into())
}

fn filled<T: Clone>(length: usize, value: T) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "allocation failed")?;
    values.resize(length, value);
    Ok(values)
}

fn finite_real(values: &[f64]) -> Result<(), String> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("nonfinite real output".into())
    }
}

fn finite_complex(values: &[Complex64]) -> Result<(), String> {
    if values
        .iter()
        .all(|value| value.re.is_finite() && value.im.is_finite())
    {
        Ok(())
    } else {
        Err("nonfinite complex output".into())
    }
}

fn hash_complex(values: &[Complex64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn hash_real(values: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn rustc_version() -> &'static str {
    option_env!("RUSTC_VERSION").unwrap_or("1.94.0")
}

fn parse(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid dimension {value}"))
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::validate_direct_dft;

    #[test]
    fn owned_and_avx_backends_match_direct_dft() {
        validate_direct_dft().expect("both FFT backends should match the independent direct DFT");
    }
}
