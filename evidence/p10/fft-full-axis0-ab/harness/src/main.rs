use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, W3FftMode, W3FftPool};
use nsbu_solver::Complex64;
#[cfg(test)]
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use std::io::Write;
use std::mem::size_of;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const N: usize = 768;
const HELPERS: usize = 8;
const ROUNDS: usize = 3;
const ALLOCATION_ALLOWANCE: usize = 64;
const HARNESS_ALLOWANCE: usize = 1 << 30;
const HASH_STACK_BYTES: usize = 64 * 1024;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let mode = args
        .get(1)
        .expect("usage: harness axis12|full [--preflight]");
    assert!(mode == "axis12" || mode == "full");
    let preflight_only = args.get(2).is_some_and(|arg| arg == "--preflight");
    let started = Instant::now();
    let layout = Layout::new([N; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let lane_bytes = FftPlan::reservation_with_shared_backend(layout, backend).unwrap();
    let w3_bytes = if mode == "full" {
        W3FftPool::additional_parallel_full_axis0_reservation_with_backend(
            layout,
            backend,
            W3FftMode::Bidirectional,
            HELPERS,
        )
        .unwrap()
    } else {
        W3FftPool::additional_parallel_reservation_with_backend(
            layout,
            backend,
            W3FftMode::Bidirectional,
            HELPERS,
        )
        .unwrap()
    };
    let external_bytes =
        6 * (layout.real_len() * size_of::<f64>() + ALLOCATION_ALLOWANCE + size_of::<Vec<f64>>());
    let seed_spectrum_bytes = layout.half_len() * size_of::<Complex64>()
        + ALLOCATION_ALLOWANCE
        + size_of::<Vec<Complex64>>();
    let resource_bytes = catalog_bytes
        + lane_bytes
        + seed_spectrum_bytes
        + w3_bytes
        + external_bytes
        + HARNESS_ALLOWANCE;
    if preflight_only {
        println!(
            "{{\"schema\":\"p10-fft-full-axis0-ab-preflight-v1\",\"mode\":\"{mode}\",\"resource_bytes\":{resource_bytes},\"w3_bytes\":{w3_bytes},\"hash_stack_bytes\":{HASH_STACK_BYTES}}}"
        );
        return;
    }
    phase("preflight", started);
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    assert_eq!(
        lane_bytes,
        FftPlan::reservation_from_catalog(layout, &catalog).unwrap()
    );
    let (seed_plan, seed_work) = FftPlan::new_from_catalog(layout, &catalog, lane_bytes).unwrap();
    let seed_spectrum = filled_complex(layout.half_len());
    let mut pool = if mode == "full" {
        W3FftPool::from_scalar_lane_parallel_full_axis0(
            layout,
            &catalog,
            W3FftMode::Bidirectional,
            HELPERS,
            (seed_plan, seed_work, seed_spectrum),
            w3_bytes,
        )
        .unwrap()
    } else {
        W3FftPool::from_scalar_lane_parallel(
            layout,
            &catalog,
            W3FftMode::Bidirectional,
            HELPERS,
            (seed_plan, seed_work, seed_spectrum),
            w3_bytes,
        )
        .unwrap()
    };
    let mut inputs: [Vec<f64>; 3] =
        std::array::from_fn(|lane| dense_input(layout.real_len(), lane));
    let mut outputs: [Vec<f64>; 3] = std::array::from_fn(|_| filled_real(layout.real_len()));
    phase("allocated", started);
    pool.forward3(&mut inputs).unwrap();
    pool.inverse3(&mut outputs).unwrap();
    phase("warm_pair", started);

    let mut forward_ns = [0_u128; ROUNDS];
    let mut inverse_ns = [0_u128; ROUNDS];
    let mut forward_hash = [[0_u8; 32]; ROUNDS];
    let mut inverse_hash = [[0_u8; 32]; ROUNDS];
    let mut allocation_totals = [0_usize; 3];
    for round in 0..ROUNDS {
        let region = Region::new(GLOBAL);
        let timer = Instant::now();
        pool.forward3(&mut inputs).unwrap();
        forward_ns[round] = timer.elapsed().as_nanos();
        let stats = region.change();
        add_stats(&mut allocation_totals, &stats);
        forward_hash[round] = hash_spectra(&pool).unwrap();
        eprintln!(
            "{{\"phase\":\"forward_result\",\"round\":{round},\"elapsed_ns\":{},\"allocations\":[{},{},{}],\"sha256\":\"{}\"}}",
            forward_ns[round], stats.allocations, stats.deallocations, stats.reallocations,
            hex(&forward_hash[round]),
        );
        phase("timed_forward", started);

        let region = Region::new(GLOBAL);
        let timer = Instant::now();
        pool.inverse3(&mut outputs).unwrap();
        inverse_ns[round] = timer.elapsed().as_nanos();
        let stats = region.change();
        add_stats(&mut allocation_totals, &stats);
        inverse_hash[round] = hash_real3(&outputs).unwrap();
        eprintln!(
            "{{\"phase\":\"inverse_result\",\"round\":{round},\"elapsed_ns\":{},\"allocations\":[{},{},{}],\"sha256\":\"{}\"}}",
            inverse_ns[round], stats.allocations, stats.deallocations, stats.reallocations,
            hex(&inverse_hash[round]),
        );
        phase("timed_inverse", started);
    }
    println!(
        "{{\"schema\":\"p10-fft-full-axis0-ab-v1\",\"mode\":\"{mode}\",\"n\":{N},\"helpers\":{HELPERS},\"persistent_callers\":3,\"rounds\":{ROUNDS},\"catalog_bytes\":{catalog_bytes},\"lane_bytes\":{lane_bytes},\"seed_spectrum_bytes\":{seed_spectrum_bytes},\"w3_bytes\":{w3_bytes},\"external_bytes\":{external_bytes},\"harness_allowance\":{HARNESS_ALLOWANCE},\"hash_stack_bytes\":{HASH_STACK_BYTES},\"resource_bytes\":{resource_bytes},\"forward_ns\":{:?},\"inverse_ns\":{:?},\"allocations\":{:?},\"forward_sha256\":[\"{}\",\"{}\",\"{}\"],\"inverse_sha256\":[\"{}\",\"{}\",\"{}\"]}}",
        forward_ns, inverse_ns, allocation_totals,
        hex(&forward_hash[0]), hex(&forward_hash[1]), hex(&forward_hash[2]),
        hex(&inverse_hash[0]), hex(&inverse_hash[1]), hex(&inverse_hash[2]),
    );
}

fn filled_complex(len: usize) -> Vec<Complex64> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).unwrap();
    values.resize(len, Complex64::new(0.0, 0.0));
    values
}

fn filled_real(len: usize) -> Vec<f64> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).unwrap();
    values.resize(len, 0.0);
    values
}

fn dense_input(len: usize, lane: usize) -> Vec<f64> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).unwrap();
    values.extend((0..len).map(|index| {
        let bits = (index as u64 ^ ((lane as u64 + 1) * 0x517c_c1b7_2722_0a95))
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(17);
        (bits >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64)) - 0.5
    }));
    values
}

fn hash_spectra(pool: &W3FftPool) -> std::io::Result<[u8; 32]> {
    let mut hash = OpenSslSha::new()?;
    for lane in 0..3 {
        pool.with_spectrum(lane, |values| {
            for value in values {
                hash.push(value.re.to_bits().to_le_bytes());
                hash.push(value.im.to_bits().to_le_bytes());
            }
        })
        .unwrap();
    }
    hash.finish()
}

fn hash_real3(values: &[Vec<f64>; 3]) -> std::io::Result<[u8; 32]> {
    let mut hash = OpenSslSha::new()?;
    for lane in values {
        for value in lane {
            hash.push(value.to_bits().to_le_bytes());
        }
    }
    hash.finish()
}

struct OpenSslSha {
    child: Child,
    stdin: Option<ChildStdin>,
    bytes: [u8; HASH_STACK_BYTES],
    used: usize,
}

impl OpenSslSha {
    fn new() -> std::io::Result<Self> {
        let mut child = Command::new("/usr/bin/openssl")
            .args(["dgst", "-sha256", "-binary"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped OpenSSL stdin");
        Ok(Self {
            child,
            stdin: Some(stdin),
            bytes: [0; HASH_STACK_BYTES],
            used: 0,
        })
    }

    fn push(&mut self, word: [u8; 8]) {
        if self.used == self.bytes.len() {
            self.flush();
        }
        self.bytes[self.used..self.used + 8].copy_from_slice(&word);
        self.used += 8;
    }

    fn flush(&mut self) {
        self.stdin
            .as_mut()
            .expect("OpenSSL stdin remains open")
            .write_all(&self.bytes[..self.used])
            .expect("write canonical hash stream to OpenSSL");
        self.used = 0;
    }

    fn finish(mut self) -> std::io::Result<[u8; 32]> {
        self.flush();
        drop(self.stdin.take());
        let output = self.child.wait_with_output()?;
        if !output.status.success() || output.stdout.len() != 32 {
            return Err(std::io::Error::other(format!(
                "OpenSSL SHA-256 failed: status={} stdout_bytes={} stderr={}",
                output.status,
                output.stdout.len(),
                String::from_utf8_lossy(&output.stderr),
            )));
        }
        Ok(output.stdout.try_into().expect("checked digest length"))
    }
}

fn add_stats(total: &mut [usize; 3], stats: &stats_alloc::Stats) {
    total[0] += stats.allocations;
    total[1] += stats.deallocations;
    total[2] += stats.reallocations;
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn phase(name: &str, started: Instant) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    eprintln!(
        "{{\"phase\":\"{name}\",\"elapsed_ns\":{},\"unix_seconds\":{},\"unix_nanos\":{}}}",
        started.elapsed().as_nanos(),
        now.as_secs(),
        now.subsec_nanos(),
    );
    std::io::stderr().flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batched_hash_matches_word_at_a_time_across_full_and_partial_buffers() {
        let words = (0..9_013_u64)
            .map(|i| i.wrapping_mul(0x9e37_79b9_7f4a_7c15).to_le_bytes())
            .collect::<Vec<_>>();
        let mut expected = Sha256::new();
        let mut actual = OpenSslSha::new().unwrap();
        for word in words {
            expected.update(word);
            actual.push(word);
        }
        let expected: [u8; 32] = expected.finalize().into();
        assert_eq!(actual.finish().unwrap(), expected);
    }
}
