#![allow(clippy::needless_range_loop)]
use crate::row_pack::{axis_rows, pack_rows, scatter_rows};
use crate::transform::{checked_product, real_len, zeros, AxisKind, Lane, Plans, Timing};
use rustfft::{num_complex::Complex64, Fft};
use std::{
    mem::size_of,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Arc, Barrier, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const COMPONENTS: usize = 3;
pub const PARTICIPANTS: usize = 4;
pub const HELPERS: usize = 3;
pub const BLOCK_ROWS: usize = 256;
pub const SLOTS: usize = 2;
pub const STACK_BYTES: usize = 2 * 1024 * 1024;
pub const THREAD_ALLOWANCE: usize = 64 * 1024;
pub const METADATA_ALLOWANCE: usize = 2_048;
pub const ALLOCATION_ALLOWANCE: usize = 64;

struct HelperData {
    buffers: [Mutex<Vec<Complex64>>; SLOTS],
    scratch: Mutex<Vec<Complex64>>,
}

struct Job {
    kind: AtomicU8,
    start: AtomicUsize,
    rows: AtomicUsize,
    slot: AtomicUsize,
    fail: AtomicBool,
    compute_ns: AtomicU64,
    failed: AtomicBool,
}

impl Job {
    fn new() -> Self {
        Self {
            kind: AtomicU8::new(0),
            start: AtomicUsize::new(0),
            rows: AtomicUsize::new(0),
            slot: AtomicUsize::new(0),
            fail: AtomicBool::new(false),
            compute_ns: AtomicU64::new(0),
            failed: AtomicBool::new(false),
        }
    }
}

struct Shared {
    start_barrier: Barrier,
    done_barrier: Barrier,
    shutdown: AtomicBool,
    jobs: [Job; HELPERS],
    helpers: [HelperData; HELPERS],
}

pub struct RowExecutor {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
    buffers: [Vec<Complex64>; SLOTS],
    slot: usize,
    inject_helper: Option<usize>,
    terminated: bool,
}

impl RowExecutor {
    pub fn new(n: usize, plans: &Plans) -> Result<Self, String> {
        let make_buffers = || -> Result<[Vec<Complex64>; SLOTS], String> {
            Ok([zeros(BLOCK_ROWS * n)?, zeros(BLOCK_ROWS * n)?])
        };
        let helpers = [0, 1, 2].map(|_| -> Result<HelperData, String> {
            Ok(HelperData {
                buffers: make_buffers()?.map(Mutex::new),
                scratch: Mutex::new(zeros(4 * n)?),
            })
        });
        let [a, b, c] = helpers;
        let shared = Arc::new(Shared {
            start_barrier: Barrier::new(PARTICIPANTS),
            done_barrier: Barrier::new(PARTICIPANTS),
            shutdown: AtomicBool::new(false),
            jobs: [Job::new(), Job::new(), Job::new()],
            helpers: [a?, b?, c?],
        });
        let mut threads = Vec::with_capacity(HELPERS);
        for worker in 0..HELPERS {
            let state = shared.clone();
            let forward = plans.forward.clone();
            let inverse = plans.inverse.clone();
            let handle = thread::Builder::new()
                .name(format!("row-avx-helper-{worker}"))
                .stack_size(STACK_BYTES)
                .spawn(move || helper_loop(worker, state, forward, inverse))
                .map_err(|error| format!("helper spawn: {error}"))?;
            threads.push(handle);
        }
        Ok(Self {
            shared,
            threads,
            buffers: make_buffers()?,
            slot: 0,
            inject_helper: None,
            terminated: false,
        })
    }

    pub fn inject_failure(&mut self, helper: usize) {
        self.inject_helper = Some(helper);
    }

    pub fn forward(&mut self, lane: &mut Lane) -> Result<Timing, String> {
        self.check()?;
        let mut timing = Timing::default();
        self.axis(lane, AxisKind::RealZ, &mut timing)?;
        self.axis(lane, AxisKind::ForwardX, &mut timing)?;
        self.axis(lane, AxisKind::ForwardY, &mut timing)?;
        let started = Instant::now();
        let scale = real_len(lane.n)? as f64;
        for (output, value) in lane.spectrum.iter_mut().zip(&lane.grid) {
            *output = *value / scale;
        }
        timing.publication += started.elapsed();
        Ok(timing)
    }

    pub fn inverse(&mut self, lane: &mut Lane) -> Result<Timing, String> {
        self.check()?;
        lane.grid.copy_from_slice(&lane.spectrum);
        let mut timing = Timing::default();
        self.axis(lane, AxisKind::InverseX, &mut timing)?;
        self.axis(lane, AxisKind::InverseY, &mut timing)?;
        self.axis(lane, AxisKind::InverseZ, &mut timing)?;
        Ok(timing)
    }

    fn check(&self) -> Result<(), String> {
        if self.terminated {
            Err("row executor permanently terminated".into())
        } else {
            Ok(())
        }
    }

    fn axis(&mut self, lane: &mut Lane, kind: AxisKind, timing: &mut Timing) -> Result<(), String> {
        let total = axis_rows(lane.n, lane.half, kind);
        let mut start = 0;
        while start < total {
            let slot = self.slot;
            let assigned = assigned_rows(total, start);
            timing.pack += self.pack_wave(lane, kind, start, slot, assigned)?;
            timing.dispatch += self.dispatch_wave(kind, start, slot, assigned);
            let (phase, pure_wait) = self.fft_wave(lane, kind, slot, assigned);
            timing.fft_phase += phase;
            timing.pure_wait += pure_wait;
            if self
                .shared
                .jobs
                .iter()
                .any(|job| job.failed.load(Ordering::Relaxed))
            {
                self.terminated = true;
                return Err(format!(
                    "injected/caught worker failure after drained wave at row {start}"
                ));
            }
            timing.scatter += self.scatter_wave(lane, kind, start, slot, assigned)?;
            timing.blocks += assigned.iter().filter(|rows| **rows != 0).count();
            start = start.saturating_add(PARTICIPANTS * BLOCK_ROWS);
            self.slot = 1 - self.slot;
        }
        Ok(())
    }

    fn pack_wave(
        &mut self,
        lane: &Lane,
        kind: AxisKind,
        start: usize,
        slot: usize,
        assigned: [usize; PARTICIPANTS],
    ) -> Result<Duration, String> {
        let started = Instant::now();
        for (participant, rows) in assigned
            .into_iter()
            .enumerate()
            .filter(|(_, rows)| *rows != 0)
        {
            let row_start = start + participant * BLOCK_ROWS;
            if participant == 0 {
                pack_rows(lane, kind, row_start, rows, &mut self.buffers[slot]);
            } else {
                let mut buffer = self.shared.helpers[participant - 1].buffers[slot]
                    .lock()
                    .map_err(|_| "slot poisoned")?;
                pack_rows(lane, kind, row_start, rows, &mut buffer);
            }
        }
        Ok(started.elapsed())
    }

    fn dispatch_wave(
        &mut self,
        kind: AxisKind,
        start: usize,
        slot: usize,
        assigned: [usize; PARTICIPANTS],
    ) -> Duration {
        let started = Instant::now();
        for (helper, job) in self.shared.jobs.iter().enumerate() {
            job.kind.store(kind as u8, Ordering::Relaxed);
            job.start
                .store(start + (helper + 1) * BLOCK_ROWS, Ordering::Relaxed);
            job.rows.store(assigned[helper + 1], Ordering::Relaxed);
            job.slot.store(slot, Ordering::Relaxed);
            job.compute_ns.store(0, Ordering::Relaxed);
            job.failed.store(false, Ordering::Relaxed);
            job.fail
                .store(self.inject_helper == Some(helper), Ordering::Relaxed);
        }
        self.inject_helper = None;
        started.elapsed()
    }

    fn fft_wave(
        &mut self,
        lane: &mut Lane,
        kind: AxisKind,
        slot: usize,
        assigned: [usize; PARTICIPANTS],
    ) -> (Duration, Duration) {
        let phase_started = Instant::now();
        self.shared.start_barrier.wait();
        let own_started = Instant::now();
        process_rows(
            &lane.forward,
            &lane.inverse,
            kind,
            assigned[0],
            &mut self.buffers[slot],
            &mut lane.scratch,
        );
        let own_compute = own_started.elapsed();
        self.shared.done_barrier.wait();
        let phase = phase_started.elapsed();
        let helper_max = self
            .shared
            .jobs
            .iter()
            .map(|job| Duration::from_nanos(job.compute_ns.load(Ordering::Relaxed)))
            .max()
            .unwrap_or_default();
        (phase, phase.saturating_sub(own_compute.max(helper_max)))
    }

    fn scatter_wave(
        &self,
        lane: &mut Lane,
        kind: AxisKind,
        start: usize,
        slot: usize,
        assigned: [usize; PARTICIPANTS],
    ) -> Result<Duration, String> {
        let started = Instant::now();
        for (participant, rows) in assigned
            .into_iter()
            .enumerate()
            .filter(|(_, rows)| *rows != 0)
        {
            let row_start = start + participant * BLOCK_ROWS;
            if participant == 0 {
                scatter_rows(lane, kind, row_start, rows, &self.buffers[slot]);
            } else {
                let buffer = self.shared.helpers[participant - 1].buffers[slot]
                    .lock()
                    .map_err(|_| "slot poisoned")?;
                scatter_rows(lane, kind, row_start, rows, &buffer);
            }
        }
        Ok(started.elapsed())
    }
}

fn assigned_rows(total: usize, start: usize) -> [usize; PARTICIPANTS] {
    std::array::from_fn(|participant| {
        total
            .saturating_sub(start + participant * BLOCK_ROWS)
            .min(BLOCK_ROWS)
    })
}

impl Drop for RowExecutor {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.start_barrier.wait();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn helper_loop(
    worker: usize,
    shared: Arc<Shared>,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
) {
    loop {
        shared.start_barrier.wait();
        if shared.shutdown.load(Ordering::Acquire) {
            break;
        }
        let job = &shared.jobs[worker];
        let result = catch_unwind(AssertUnwindSafe(|| {
            if job.fail.load(Ordering::Relaxed) {
                panic!("injected row worker failure");
            }
            let rows = job.rows.load(Ordering::Relaxed);
            if rows == 0 {
                return;
            }
            let slot = job.slot.load(Ordering::Relaxed);
            let kind = decode_kind(job.kind.load(Ordering::Relaxed));
            let mut buffer = shared.helpers[worker].buffers[slot]
                .lock()
                .expect("slot lock");
            let mut scratch = shared.helpers[worker].scratch.lock().expect("scratch lock");
            let started = Instant::now();
            process_rows(&forward, &inverse, kind, rows, &mut buffer, &mut scratch);
            job.compute_ns
                .store(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }));
        job.failed.store(result.is_err(), Ordering::Relaxed);
        shared.done_barrier.wait();
    }
}

fn decode_kind(value: u8) -> AxisKind {
    match value {
        0 => AxisKind::RealZ,
        1 => AxisKind::ForwardX,
        2 => AxisKind::ForwardY,
        3 => AxisKind::InverseX,
        4 => AxisKind::InverseY,
        5 => AxisKind::InverseZ,
        _ => unreachable!(),
    }
}

fn process_rows(
    forward: &Arc<dyn Fft<f64>>,
    inverse: &Arc<dyn Fft<f64>>,
    kind: AxisKind,
    rows: usize,
    buffer: &mut [Complex64],
    scratch: &mut [Complex64],
) {
    let n = forward.len();
    let plan = if matches!(
        kind,
        AxisKind::InverseX | AxisKind::InverseY | AxisKind::InverseZ
    ) {
        inverse
    } else {
        forward
    };
    for row in 0..rows {
        plan.process_with_scratch(&mut buffer[row * n..(row + 1) * n], scratch);
    }
}

pub fn dispatches_per_component(n: usize) -> usize {
    let half = n / 2 + 1;
    div_ceil(n * n, BLOCK_ROWS) + 2 * div_ceil(n * half, BLOCK_ROWS)
}

pub fn reservation_delta(n: usize) -> Result<Reservation, String> {
    let slots = checked_product(&[
        2,
        COMPONENTS,
        PARTICIPANTS,
        BLOCK_ROWS,
        n,
        size_of::<Complex64>(),
    ])?;
    let scratch = checked_product(&[COMPONENTS, PARTICIPANTS - 1, 4, n, size_of::<Complex64>()])?;
    let helpers = checked_product(&[
        COMPONENTS,
        PARTICIPANTS - 1,
        STACK_BYTES + THREAD_ALLOWANCE + METADATA_ALLOWANCE,
    ])?;
    let coordinator = COMPONENTS * (METADATA_ALLOWANCE - 944);
    let allocations = 3 * COMPONENTS * PARTICIPANTS * ALLOCATION_ALLOWANCE;
    let shared_arc_payload =
        COMPONENTS * (size_of::<Shared>() + 2 * size_of::<usize>() + ALLOCATION_ALLOWANCE);
    let delta = [
        slots,
        scratch,
        helpers,
        coordinator,
        allocations,
        shared_arc_payload,
    ]
    .into_iter()
    .try_fold(0usize, |sum, value| {
        sum.checked_add(value).ok_or("reservation overflow")
    })?;
    Ok(Reservation {
        n,
        slots,
        scratch,
        helpers,
        coordinator,
        allocations,
        shared_arc_payload,
        delta,
        dispatches: COMPONENTS * dispatches_per_component(n),
    })
}

pub struct Reservation {
    pub n: usize,
    pub slots: usize,
    pub scratch: usize,
    pub helpers: usize,
    pub coordinator: usize,
    pub allocations: usize,
    pub shared_arc_payload: usize,
    pub delta: usize,
    pub dispatches: usize,
}

impl std::fmt::Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reservation n={} slots_bytes={} helper_scratch_bytes={} helper_stack_system_metadata_bytes={} coordinator_metadata_bytes={} vector_allocation_allowance_bytes={} shared_arc_payload_control_allowance_bytes={} exact_incremental_bytes={} dispatches_per_3d_triplet={} arc_plan_clones_heap_allocations=0 shared_plan_control_blocks=2 shared_arc_allocations_per_triplet=3", self.n, self.slots, self.scratch, self.helpers, self.coordinator, self.allocations, self.shared_arc_payload, self.delta, self.dispatches)
    }
}

fn div_ceil(a: usize, b: usize) -> usize {
    a / b + usize::from(!a.is_multiple_of(b))
}
