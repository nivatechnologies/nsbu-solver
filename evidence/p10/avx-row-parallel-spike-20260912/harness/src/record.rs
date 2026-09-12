use crate::{
    owner::{Audit, Mode},
    transform::{Snapshot, Timing},
};
use stats_alloc::Stats;
use std::time::Duration;

pub struct PhaseRecord {
    pub wall_seconds: f64,
    pub pack_seconds: f64,
    pub dispatch_seconds: f64,
    pub fft_phase_seconds: f64,
    pub pure_wait_seconds: f64,
    pub scatter_seconds: f64,
    pub publication_seconds: f64,
    pub overhead_seconds: f64,
    pub blocks: usize,
}

impl PhaseRecord {
    fn new(wall: Duration, timing: Timing) -> Self {
        Self {
            wall_seconds: wall.as_secs_f64(),
            pack_seconds: timing.pack.as_secs_f64(),
            dispatch_seconds: timing.dispatch.as_secs_f64(),
            fft_phase_seconds: timing.fft_phase.as_secs_f64(),
            pure_wait_seconds: timing.pure_wait.as_secs_f64(),
            scatter_seconds: timing.scatter.as_secs_f64(),
            publication_seconds: timing.publication.as_secs_f64(),
            overhead_seconds: timing.overhead().as_secs_f64(),
            blocks: timing.blocks,
        }
    }
}

pub struct ProfileResult {
    pub n: usize,
    pub mode: Mode,
    pub forward: PhaseRecord,
    pub inverse: PhaseRecord,
    pub forward_snapshot: Vec<Snapshot>,
    pub inverse_snapshot: Vec<Snapshot>,
    pub steady: Stats,
    pub audit: Audit,
    pub direct_error: Option<f64>,
}

impl ProfileResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        n: usize,
        mode: Mode,
        forward_wall: Duration,
        inverse_wall: Duration,
        forward_timing: Timing,
        inverse_timing: Timing,
        forward_snapshot: Vec<Snapshot>,
        inverse_snapshot: Vec<Snapshot>,
        steady: Stats,
        audit: Audit,
        direct_error: Option<f64>,
    ) -> Self {
        Self {
            n,
            mode,
            forward: PhaseRecord::new(forward_wall, forward_timing),
            inverse: PhaseRecord::new(inverse_wall, inverse_timing),
            forward_snapshot,
            inverse_snapshot,
            steady,
            audit,
            direct_error,
        }
    }
}

pub fn compare_snapshots(left: &[Snapshot], right: &[Snapshot], label: &str) -> Result<(), String> {
    if left.len() != right.len() {
        return Err(format!("{label}: component count mismatch"));
    }
    for (component, (a, b)) in left.iter().zip(right).enumerate() {
        if a.spectrum_hash != b.spectrum_hash || a.physical_hash != b.physical_hash {
            return Err(format!("{label}: hash mismatch component={component}"));
        }
        if let (Some(ax), Some(bx)) = (&a.spectrum, &b.spectrum) {
            if ax.len() != bx.len()
                || ax.iter().zip(bx).any(|(x, y)| {
                    x.re.to_bits() != y.re.to_bits() || x.im.to_bits() != y.im.to_bits()
                })
            {
                return Err(format!(
                    "{label}: direct spectrum word mismatch component={component}"
                ));
            }
        }
        if let (Some(ax), Some(bx)) = (&a.physical, &b.physical) {
            if ax.len() != bx.len() || ax.iter().zip(bx).any(|(x, y)| x.to_bits() != y.to_bits()) {
                return Err(format!(
                    "{label}: direct physical word mismatch component={component}"
                ));
            }
        }
    }
    Ok(())
}

pub fn print_result(result: &ProfileResult) {
    println!("{}", result.audit);
    print_phase(result, "forward", &result.forward, &result.forward_snapshot);
    print_phase(result, "inverse", &result.inverse, &result.inverse_snapshot);
    println!("steady n={} mode={:?} allocations={} deallocations={} reallocations={} bytes_allocated={} bytes_deallocated={} bytes_reallocated={}", result.n, result.mode, result.steady.allocations, result.steady.deallocations, result.steady.reallocations, result.steady.bytes_allocated, result.steady.bytes_deallocated, result.steady.bytes_reallocated);
}

fn print_phase(
    result: &ProfileResult,
    direction: &str,
    phase: &PhaseRecord,
    snapshots: &[Snapshot],
) {
    let hashes = snapshots
        .iter()
        .map(|snapshot| hex(&snapshot.spectrum_hash))
        .collect::<Vec<_>>()
        .join(",");
    let physical = snapshots
        .iter()
        .map(|snapshot| hex(&snapshot.physical_hash))
        .collect::<Vec<_>>()
        .join(",");
    println!("phase n={} mode={:?} direction={} wall_seconds={:.9} pack_seconds={:.9} dispatch_seconds={:.9} fft_barrier_phase_seconds={:.9} pure_wait_seconds={:.9} scatter_seconds={:.9} publication_seconds={:.9} measured_non_fft_overhead_seconds={:.9} dispatched_blocks={} spectrum_sha256={} physical_sha256={}", result.n, result.mode, direction, phase.wall_seconds, phase.pack_seconds, phase.dispatch_seconds, phase.fft_phase_seconds, phase.pure_wait_seconds, phase.scatter_seconds, phase.publication_seconds, phase.overhead_seconds, phase.blocks, hashes, physical);
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

pub fn sum_stats(a: Stats, b: Stats) -> Stats {
    Stats {
        allocations: a.allocations + b.allocations,
        deallocations: a.deallocations + b.deallocations,
        reallocations: a.reallocations + b.reallocations,
        bytes_allocated: a.bytes_allocated + b.bytes_allocated,
        bytes_deallocated: a.bytes_deallocated + b.bytes_deallocated,
        bytes_reallocated: a.bytes_reallocated + b.bytes_reallocated,
    }
}
