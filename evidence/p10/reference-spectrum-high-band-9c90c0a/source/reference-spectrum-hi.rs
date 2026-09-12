use nsbu_benchmarks::{CASE_SHA256, scalar, time::BenchmarkTime};
use nsbu_solver::{
    Complex64,
    domain::{Layout, TickClock, validate_spectrum},
    spectral::{FftPlan, transfer},
};
use sha2::{Digest, Sha256};
use std::{
    env,
    f64::consts::TAU,
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};

const OWNER_CAP: usize = 3 * 1024 * 1024 * 1024;
const EXPECTED_CASE: &str = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    let m: usize = args
        .get(1)
        .ok_or("usage: p10-reference-spectrum-hi M [--preflight|--run]")?
        .parse()
        .map_err(debug)?;
    let run = args.get(2).is_some_and(|a| a == "--run");
    if args.len() != 3 || (!run && args[2] != "--preflight") || ![192, 384].contains(&m) {
        return Err("usage: p10-reference-spectrum-hi {192|384} {--preflight|--run}".into());
    }
    require_identity()?;
    let samples = Layout::new([m; 3]).map_err(debug)?;
    let retained = Layout::new([192; 3]).map_err(debug)?;
    let reservation = reservation(samples, retained)?;
    println!(
        "preflight case_sha256={CASE_SHA256} clock_exponent=-20 target=8192 elapsed=4096 remaining=4096 samples={m} evaluations={} root_iteration_bound={} transforms=3 retained=192 owner_cap={OWNER_CAP} owner_reservation={reservation}",
        samples.real_len(),
        samples.real_len().checked_mul(128).ok_or("work overflow")?
    );
    normalization_control(samples)?;
    reference_control()?;
    if !run {
        return Ok(());
    }
    let start = Instant::now();
    let time = BenchmarkTime::new(TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?)
        .map_err(debug)?;
    let mut physical = [
        vec![0.0; samples.real_len()],
        vec![0.0; samples.real_len()],
        vec![0.0; samples.real_len()],
    ];
    let [nx, ny, nz] = samples.dimensions();
    let mut roots = 0usize;
    let mut root_iterations = 0usize;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let q = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    k as f64 / nz as f64,
                ];
                let value = scalar::evaluate(q, time).map_err(debug)?;
                let index = (i * ny + j) * nz + k;
                for (c, component) in physical.iter_mut().enumerate() {
                    component[index] = value.velocity[c];
                }
                if let Some(root) = value.root {
                    roots += 1;
                    root_iterations = root_iterations
                        .checked_add(root.iterations)
                        .ok_or("work overflow")?;
                }
            }
        }
    }
    if physical.iter().flatten().any(|x| !x.is_finite()) {
        return Err("nonfinite physical reference".into());
    }
    let evaluated_seconds = start.elapsed().as_secs_f64();
    let (fft, mut workspace) = FftPlan::new(samples, OWNER_CAP).map_err(debug)?;
    let mut output = vec![Complex64::new(0.0, 0.0); samples.half_len()];
    let mut spectra = [
        vec![Complex64::new(0.0, 0.0); retained.half_len()],
        vec![Complex64::new(0.0, 0.0); retained.half_len()],
        vec![Complex64::new(0.0, 0.0); retained.half_len()],
    ];
    for c in 0..3 {
        fft.forward(&physical[c], &mut output, &mut workspace)
            .map_err(debug)?;
        let audit = audit_fft_output(samples, &output)?;
        println!(
            "fft_audit samples={m} component={c} finite={} max_k0_hermitian_defect={} discarded_nyquist_l2={} discarded_nyquist_max={}",
            audit.finite, audit.max_k0_defect, audit.nyquist_l2, audit.nyquist_max
        );
        if !audit.finite || audit.max_k0_defect > 1e-12 {
            return Err(format!(
                "unadmitted raw FFT mismatch component={c} audit={audit:?}"
            ));
        }
        transfer(samples, retained, &output, &mut spectra[c]).map_err(debug)?;
        validate_spectrum(retained, &spectra[c], 1e-12).map_err(debug)?;
    }
    let total_seconds = start.elapsed().as_secs_f64();
    let path = format!("work/p10-reference-spectrum/reference-m{m}-clock4096-n192.coeff.bin");
    write_spectrum(&path, &spectra)?;
    println!(
        "result samples={m} roots={roots} root_iterations={root_iterations} evaluated_seconds={evaluated_seconds:.9} total_seconds={total_seconds:.9} retained_path={path} retained_bytes={} retained_sha256={}",
        retained.half_len() * 3 * size_of::<Complex64>(),
        digest(&spectra)
    );
    for (name, value) in shells(retained, &spectra)? {
        println!(
            "shell={name} l2={} h1={} vorticity={} divergence={}",
            value.l2, value.h1, value.vorticity, value.divergence
        );
    }
    Ok(())
}

fn require_identity() -> Result<(), String> {
    if CASE_SHA256 != EXPECTED_CASE {
        return Err("case identity mismatch".into());
    }
    let clock = TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?;
    let time = BenchmarkTime::new(clock).map_err(debug)?;
    if clock.remaining() != 4096
        || time.elapsed().to_bits() != (1.0_f64 / 256.0).to_bits()
        || time.remaining().to_bits() != (1.0_f64 / 256.0).to_bits()
    {
        return Err("clock/time identity mismatch".into());
    }
    Ok(())
}

fn reservation(samples: Layout, retained: Layout) -> Result<usize, String> {
    let real = samples
        .real_len()
        .checked_mul(3 * size_of::<f64>())
        .ok_or("size overflow")?;
    let output = samples
        .half_len()
        .checked_mul(size_of::<Complex64>())
        .ok_or("size overflow")?;
    let retained_buffers = retained
        .half_len()
        .checked_mul(3 * size_of::<Complex64>())
        .ok_or("size overflow")?;
    let fft = FftPlan::reservation(samples).map_err(debug)?;
    // Six Vec allocations, result accumulators and allocator metadata receive 64 KiB.
    let total = real
        .checked_add(output)
        .and_then(|x| x.checked_add(retained_buffers))
        .and_then(|x| x.checked_add(fft))
        .and_then(|x| x.checked_add(65536))
        .ok_or("size overflow")?;
    if total > OWNER_CAP {
        return Err(format!("owner reservation {total} exceeds {OWNER_CAP}"));
    }
    Ok(total)
}

fn normalization_control(layout: Layout) -> Result<(), String> {
    let [nx, ny, nz] = layout.dimensions();
    let mut input = vec![0.0; layout.real_len()];
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                input[(i * ny + j) * nz + k] = (TAU
                    * (i as f64 / nx as f64
                        + 2.0 * j as f64 / ny as f64
                        + 3.0 * k as f64 / nz as f64))
                    .cos();
            }
        }
    }
    let (fft, mut workspace) = FftPlan::new(layout, OWNER_CAP).map_err(debug)?;
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    fft.forward(&input, &mut spectrum, &mut workspace)
        .map_err(debug)?;
    let audit = audit_fft_output(layout, &spectrum)?;
    println!(
        "control=normalization_raw_audit samples={} audit={audit:?}",
        nx
    );
    if !audit.finite || audit.max_k0_defect > 1e-12 {
        return Err(format!("normalization raw FFT mismatch {audit:?}"));
    }
    let mut strict = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    transfer(layout, layout, &spectrum, &mut strict).map_err(debug)?;
    validate_spectrum(layout, &strict, 1e-12).map_err(debug)?;
    spectrum = strict;
    let (index, conjugate) = layout.locate([1, 2, 3]).map_err(debug)?;
    if conjugate
        || (spectrum[index] - Complex64::new(0.5, 0.0))
            .norm_sqr()
            .sqrt()
            > 256.0 * f64::EPSILON
    {
        return Err(format!(
            "signed-mode normalization mismatch {:?}",
            spectrum[index]
        ));
    }
    let mut other = 0.0_f64;
    for (p, value) in spectrum.iter().enumerate() {
        if p != index {
            other = other.max(value.norm_sqr().sqrt());
        }
    }
    if other > 1024.0 * f64::EPSILON {
        return Err(format!("signed-mode leakage {other}"));
    }
    println!(
        "control=normalization samples={} mode=[1,2,3] coefficient={:?} max_other={other}",
        nx, spectrum[index]
    );
    Ok(())
}

fn reference_control() -> Result<(), String> {
    let time = BenchmarkTime::new(TickClock::restore(-20, 8192, 4096, 4096).map_err(debug)?)
        .map_err(debug)?;
    for point in [[0.0, 0.0, 0.0], [0.125, -0.125, 0.0], [0.49, 0.49, 0.49]] {
        let scalar = scalar::evaluate(point, time).map_err(debug)?;
        let jet = nsbu_benchmarks::fields::reference::evaluate(point, time).map_err(debug)?;
        for c in 0..3 {
            let scale = scalar.velocity[c].abs().max(jet.velocity[c].abs()).max(1.0);
            if (scalar.velocity[c] - jet.velocity[c]).abs() > 4096.0 * f64::EPSILON * scale {
                return Err(format!("scalar/jet mismatch point={point:?} component={c}"));
            }
        }
    }
    println!("control=scalar_jet points=3 status=passed");
    Ok(())
}

#[derive(Debug)]
struct Norms {
    l2: f64,
    h1: f64,
    vorticity: f64,
    divergence: f64,
}

#[derive(Debug)]
struct FftAudit {
    finite: bool,
    max_k0_defect: f64,
    nyquist_l2: f64,
    nyquist_max: f64,
}
fn audit_fft_output(layout: Layout, spectrum: &[Complex64]) -> Result<FftAudit, String> {
    if spectrum.len() != layout.half_len() {
        return Err("FFT output length mismatch".into());
    }
    let [nx, ny, _] = layout.dimensions();
    let mut finite = true;
    let mut max_k0_defect = 0.0_f64;
    let mut nyquist2 = 0.0;
    let mut nyquist_max = 0.0_f64;
    for (index, _) in spectrum.iter().enumerate() {
        let p = layout.position(index).map_err(debug)?;
        let z = spectrum[index];
        finite &= z.re.is_finite() && z.im.is_finite();
        if layout.is_nyquist(p).map_err(debug)? {
            nyquist2 += z.norm_sqr();
            nyquist_max = nyquist_max.max(z.norm_sqr().sqrt());
        }
        if p[2] == 0 {
            let q = layout
                .index([(nx - p[0]) % nx, (ny - p[1]) % ny, 0])
                .map_err(debug)?;
            max_k0_defect = max_k0_defect.max((z - spectrum[q].conj()).norm_sqr().sqrt());
        }
    }
    Ok(FftAudit {
        finite,
        max_k0_defect,
        nyquist_l2: nyquist2.sqrt(),
        nyquist_max,
    })
}
fn shells(
    layout: Layout,
    spectra: &[Vec<Complex64>; 3],
) -> Result<Vec<(&'static str, Norms)>, String> {
    Ok(vec![
        ("inside_n48", norm_range(layout, spectra, 0, 48)?),
        ("n48_to_n64", norm_range(layout, spectra, 48, 64)?),
        ("n64_to_n96", norm_range(layout, spectra, 64, 96)?),
        ("n96_to_n128", norm_range(layout, spectra, 96, 128)?),
        ("n128_to_n192", norm_range(layout, spectra, 128, 192)?),
        ("inside_n192", norm_range(layout, spectra, 0, 192)?),
    ])
}
fn norm_range(
    layout: Layout,
    spectra: &[Vec<Complex64>; 3],
    low: usize,
    high: usize,
) -> Result<Norms, String> {
    let mut l2 = 0.0;
    let mut h1 = 0.0;
    let mut vort = 0.0;
    let mut div = 0.0;
    for (index, _) in spectra[0].iter().enumerate() {
        let pos = layout.position(index).map_err(debug)?;
        let mode = layout.mode(pos).map_err(debug)?;
        let inside = |n: usize| mode.iter().all(|x| x.unsigned_abs() < n / 2);
        if !inside(high) || (low > 0 && inside(low)) {
            continue;
        }
        let w = layout.weight(pos).map_err(debug)?;
        if w == 0.0 {
            continue;
        }
        let k = mode.map(|x| TAU * x as f64);
        let u = [spectra[0][index], spectra[1][index], spectra[2][index]];
        let e = u.iter().map(|z| z.norm_sqr()).sum::<f64>();
        let k2 = k.iter().map(|x| x * x).sum::<f64>();
        l2 += w * e;
        h1 += w * (1.0 + k2) * e;
        let d = u[0] * k[0] + u[1] * k[1] + u[2] * k[2];
        div += w * d.norm_sqr();
        let c = [
            u[2] * k[1] - u[1] * k[2],
            u[0] * k[2] - u[2] * k[0],
            u[1] * k[0] - u[0] * k[1],
        ];
        vort += w * c.iter().map(|z| z.norm_sqr()).sum::<f64>();
    }
    Ok(Norms {
        l2: l2.sqrt(),
        h1: h1.sqrt(),
        vorticity: vort.sqrt(),
        divergence: div.sqrt(),
    })
}
fn digest(spectra: &[Vec<Complex64>; 3]) -> String {
    let mut h = Sha256::new();
    for s in spectra {
        for z in s {
            h.update(z.re.to_bits().to_le_bytes());
            h.update(z.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", h.finalize())
}
fn write_spectrum(path: &str, spectra: &[Vec<Complex64>; 3]) -> Result<(), String> {
    let mut output = BufWriter::with_capacity(8192, File::create(path).map_err(debug)?);
    for component in spectra {
        for value in component {
            output
                .write_all(&value.re.to_bits().to_le_bytes())
                .map_err(debug)?;
            output
                .write_all(&value.im.to_bits().to_le_bytes())
                .map_err(debug)?;
        }
    }
    output.flush().map_err(debug)
}
fn debug<E: std::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}
