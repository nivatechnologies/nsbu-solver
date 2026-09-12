use nsbu_benchmarks::{scalar, time::BenchmarkTime, CASE_SHA256};
use nsbu_solver::{
    domain::{validate_spectrum, Layout, TickClock},
    spectral::{transfer, FftPlan},
    Complex64,
};
use sha2::{Digest, Sha256};
use std::{env, f64::consts::TAU, fs};

const N: usize = 192;
const M: usize = 384;
const CAP: usize = 4 * 1024 * 1024 * 1024;
const INPUT_SHA256: &str = "f2c080900c828b88494ea0e1912a7acfe106637668701bea26b88d3bab6e039f";
const IDENTITY: &str = "source=codex/p10-fft-batch-20260912@92effa6068d2;case=e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e;backend=rustfft-6.4.1-avx-avx2-fma;provider=parallel-reduced-attempt-cache;n=192;m=384;workers=32;method=cox-matthews;step=32;endpoint=4096;advective_limit=0.45;cap=103079215104;schema=p10-avx-scheduled-endpoint-v2;resume=unsupported";

fn main() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: p10-actual-state-smoke STATE")?;
    if env::args().nth(2).is_some() {
        return Err("usage: p10-actual-state-smoke STATE".into());
    }
    let retained = Layout::new([N; 3]).map_err(debug)?;
    let samples = Layout::new([M; 3]).map_err(debug)?;
    let reservation = reservation(retained, samples)?;
    println!("preflight source=92effa6068d20d69e815c6a83f1e82490ce37fe7 case_sha256={CASE_SHA256} state_sha256={INPUT_SHA256} retained={N} reference_samples={M} clock=512 owner_cap={CAP} owner_reservation={reservation} precision=binary64_scalar_reference_not_independent_high_precision");
    let state = load(&path, retained)?;
    let reference = reference(samples, retained)?;
    let error = norms(retained, &state, &reference)?;
    println!("result snapshot={} state_sha256={} reference_sampling={M} full_l2={} full_h1={} full_vorticity={} full_divergence={} common_equals_full=true newly_resolved=not_applicable_same_n192 scope=gross_tracking_smoke_only", path, INPUT_SHA256, error.l2, error.h1, error.vorticity, error.divergence);
    Ok(())
}

fn reservation(retained: Layout, samples: Layout) -> Result<usize, String> {
    let state = retained
        .half_len()
        .checked_mul(3 * 16)
        .ok_or("size overflow")?;
    let physical = samples
        .real_len()
        .checked_mul(3 * 8)
        .ok_or("size overflow")?;
    let output = samples.half_len().checked_mul(16).ok_or("size overflow")?;
    let reference = state;
    let fft = FftPlan::reservation(samples).map_err(debug)?;
    let total = state
        .checked_mul(2)
        .and_then(|x| x.checked_add(physical))
        .and_then(|x| x.checked_add(output))
        .and_then(|x| x.checked_add(reference))
        .and_then(|x| x.checked_add(fft))
        .and_then(|x| x.checked_add(65536))
        .ok_or("size overflow")?;
    if total > CAP {
        return Err("owner reservation exceeds cap".into());
    }
    Ok(total)
}

fn load(path: &str, layout: Layout) -> Result<[Vec<Complex64>; 3], String> {
    let bytes = fs::read(path).map_err(debug)?;
    let mut cursor = Cursor::new(&bytes);
    if cursor.take(12)? != b"P10AVXSNAP1\0" {
        return Err("snapshot magic mismatch".into());
    }
    let id_len = cursor.u64()? as usize;
    let identity = std::str::from_utf8(cursor.take(id_len)?).map_err(debug)?;
    if identity != IDENTITY {
        return Err("snapshot identity mismatch".into());
    }
    if [
        cursor.u128()?,
        cursor.u128()?,
        cursor.u128()?,
        cursor.u128()?,
    ] != [512, 8192, 16, 16]
    {
        return Err("snapshot clock header mismatch".into());
    }
    let payload_len = layout
        .half_len()
        .checked_mul(3 * 16)
        .ok_or("size overflow")?;
    let payload = cursor.take(payload_len)?;
    let digest = cursor.take(32)?;
    if cursor.remaining() != 0 {
        return Err("snapshot trailing bytes".into());
    }
    let hash = Sha256::digest(payload);
    if hash.as_slice() != digest || format!("{hash:x}") != INPUT_SHA256 {
        return Err("snapshot payload hash mismatch".into());
    }
    let values = std::array::from_fn(|axis| {
        decode(&payload[axis * layout.half_len() * 16..][..layout.half_len() * 16])
    });
    for component in &values {
        validate_spectrum(layout, component, 1e-12).map_err(debug)?;
    }
    Ok(values)
}

fn decode(bytes: &[u8]) -> Vec<Complex64> {
    bytes
        .chunks_exact(16)
        .map(|word| {
            Complex64::new(
                f64::from_bits(u64::from_le_bytes(word[..8].try_into().unwrap())),
                f64::from_bits(u64::from_le_bytes(word[8..].try_into().unwrap())),
            )
        })
        .collect()
}

fn reference(samples: Layout, retained: Layout) -> Result<[Vec<Complex64>; 3], String> {
    let time = BenchmarkTime::new(TickClock::restore(-20, 8192, 512, 7680).map_err(debug)?)
        .map_err(debug)?;
    let mut physical = [
        vec![0.0; samples.real_len()],
        vec![0.0; samples.real_len()],
        vec![0.0; samples.real_len()],
    ];
    let [nx, ny, nz] = samples.dimensions();
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let index = (i * ny + j) * nz + k;
                let value = scalar::evaluate(
                    [
                        i as f64 / nx as f64,
                        j as f64 / ny as f64,
                        k as f64 / nz as f64,
                    ],
                    time,
                )
                .map_err(debug)?;
                for (axis, component) in physical.iter_mut().enumerate() {
                    component[index] = value.velocity[axis];
                }
            }
        }
    }
    let (fft, mut workspace) = FftPlan::new(samples, CAP).map_err(debug)?;
    let mut output = vec![Complex64::new(0.0, 0.0); samples.half_len()];
    let mut result = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); retained.half_len()]);
    for axis in 0..3 {
        fft.forward(&physical[axis], &mut output, &mut workspace)
            .map_err(debug)?;
        transfer(samples, retained, &output, &mut result[axis]).map_err(debug)?;
        validate_spectrum(retained, &result[axis], 1e-12).map_err(debug)?;
    }
    Ok(result)
}

struct Norms {
    l2: f64,
    h1: f64,
    vorticity: f64,
    divergence: f64,
}
fn norms(
    layout: Layout,
    left: &[Vec<Complex64>; 3],
    right: &[Vec<Complex64>; 3],
) -> Result<Norms, String> {
    let mut l2 = 0.0;
    let mut h1 = 0.0;
    let mut vort = 0.0;
    let mut div = 0.0;
    for index in 0..layout.half_len() {
        let position = layout.position(index).map_err(debug)?;
        let weight = layout.weight(position).map_err(debug)?;
        if weight == 0.0 {
            continue;
        }
        let k = layout
            .mode(position)
            .map_err(debug)?
            .map(|x| TAU * x as f64);
        let u: [Complex64; 3] = std::array::from_fn(|axis| left[axis][index] - right[axis][index]);
        let energy = u.iter().map(|z| z.norm_sqr()).sum::<f64>();
        let k2 = k.iter().map(|x| x * x).sum::<f64>();
        let d = u[0] * k[0] + u[1] * k[1] + u[2] * k[2];
        let c = [
            u[2] * k[1] - u[1] * k[2],
            u[0] * k[2] - u[2] * k[0],
            u[1] * k[0] - u[0] * k[1],
        ];
        l2 += weight * energy;
        h1 += weight * (1.0 + k2) * energy;
        div += weight * d.norm_sqr();
        vort += weight * c.iter().map(|z| z.norm_sqr()).sum::<f64>();
    }
    Ok(Norms {
        l2: l2.sqrt(),
        h1: h1.sqrt(),
        vorticity: vort.sqrt(),
        divergence: div.sqrt(),
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.offset.checked_add(count).ok_or("snapshot overflow")?;
        let part = self
            .bytes
            .get(self.offset..end)
            .ok_or("snapshot truncated")?;
        self.offset = end;
        Ok(part)
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(debug)?))
    }
    fn u128(&mut self) -> Result<u128, String> {
        Ok(u128::from_le_bytes(
            self.take(16)?.try_into().map_err(debug)?,
        ))
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}
fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}
