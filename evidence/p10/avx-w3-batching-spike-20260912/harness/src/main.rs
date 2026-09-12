use nsbu_solver::{
    domain::Layout,
    spectral::{FftBackend, FftCatalog, FftPlan, FftWorkspace},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    hint::black_box,
    mem::size_of,
    process::ExitCode,
    sync::{Arc, Condvar, Mutex},
    thread::{Builder, JoinHandle},
    time::{Duration, Instant},
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const WIDTH: usize = 3;
const STACK_BYTES: usize = 2 * 1024 * 1024;
const THREAD_ALLOWANCE: usize = 64 * 1024;
const ALLOCATION_ALLOWANCE: usize = 64;
const LENGTHS: [usize; 3] = [288, 384, 576];

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
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().map_err(debug)?;
    println!(
        "identity source={} backend=rustfft-6.4.1-avx-avx2-fma width={WIDTH} runtime_avx={} runtime_avx2={} runtime_fma={} publication=drain_then_lane_order_swap",
        option_env!("SOURCE_ID").unwrap_or("uncommitted-spike"),
        std::is_x86_feature_detected!("avx"),
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("fma"),
    );
    let catalog_bytes = FftCatalog::reservation(backend).map_err(debug)?;
    let catalog = FftCatalog::new(backend, catalog_bytes).map_err(debug)?;
    direct_reference_and_failure_controls(&catalog)?;
    for length in LENGTHS {
        profile(length, &catalog)?;
    }
    println!("terminal=profile-complete");
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Forward,
    Inverse,
    Noop,
}

struct Lane {
    fft: FftPlan,
    work: FftWorkspace,
    physical: Vec<f64>,
    spectrum: Vec<Complex64>,
    staged_physical: Vec<f64>,
    staged_spectrum: Vec<Complex64>,
}

impl Lane {
    fn new(layout: Layout, catalog: &FftCatalog) -> Result<Self, String> {
        let bytes = FftPlan::reservation_from_catalog(layout, catalog).map_err(debug)?;
        let (fft, work) = FftPlan::new_from_catalog(layout, catalog, bytes).map_err(debug)?;
        Ok(Self {
            fft,
            work,
            physical: filled(layout.real_len(), 0.0)?,
            spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
            staged_physical: filled(layout.real_len(), 0.0)?,
            staged_spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        })
    }

    fn reset(&mut self, variant: usize) {
        fill_input(&mut self.physical, variant);
        self.spectrum.fill(Complex64::new(0.0, 0.0));
        self.staged_physical.fill(0.0);
        self.staged_spectrum.fill(Complex64::new(0.0, 0.0));
    }

    fn run(&mut self, operation: Operation) -> Result<(), SolverError> {
        match operation {
            Operation::Forward => {
                self.fft
                    .forward(&self.physical, &mut self.staged_spectrum, &mut self.work)
            }
            Operation::Inverse => {
                self.fft
                    .inverse(&self.spectrum, &mut self.staged_physical, &mut self.work)
            }
            Operation::Noop => Ok(()),
        }
    }

    fn publish(&mut self, operation: Operation) {
        match operation {
            Operation::Forward => std::mem::swap(&mut self.spectrum, &mut self.staged_spectrum),
            Operation::Inverse => std::mem::swap(&mut self.physical, &mut self.staged_physical),
            Operation::Noop => {}
        }
    }
}

struct SerialBatch {
    lanes: Vec<Lane>,
}

impl SerialBatch {
    fn new(layout: Layout, catalog: &FftCatalog) -> Result<Self, String> {
        let mut lanes = Vec::new();
        lanes
            .try_reserve_exact(WIDTH)
            .map_err(|_| "serial lane allocation")?;
        for _ in 0..WIDTH {
            lanes.push(Lane::new(layout, catalog)?);
        }
        Ok(Self { lanes })
    }

    fn reset(&mut self, base: usize) {
        for (lane, value) in self.lanes.iter_mut().zip(base..) {
            lane.reset(value);
        }
    }

    fn execute(&mut self, operation: Operation) -> Result<(), String> {
        for lane in &mut self.lanes {
            lane.run(operation).map_err(debug)?;
        }
        for lane in &mut self.lanes {
            lane.publish(operation);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkerFailure {
    Numerical,
    Injected,
    Panic,
    Protocol,
}

struct WorkerState {
    started: bool,
    pending: Option<Operation>,
    complete: Option<Result<(), WorkerFailure>>,
    stop: bool,
    fail_next: bool,
    panic_next: bool,
    lane: Lane,
}

struct Shared {
    state: Mutex<WorkerState>,
    ready: Condvar,
    done: Condvar,
}

struct Worker {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(lane: Lane) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            state: Mutex::new(WorkerState {
                started: false,
                pending: None,
                complete: None,
                stop: false,
                fail_next: false,
                panic_next: false,
                lane,
            }),
            ready: Condvar::new(),
            done: Condvar::new(),
        });
        let thread = Arc::clone(&shared);
        let handle = Builder::new()
            .stack_size(STACK_BYTES)
            .spawn(move || worker_loop(&thread))
            .map_err(|_| "worker spawn failed")?;
        let worker = Self {
            shared,
            handle: Some(handle),
        };
        worker.wait_started()?;
        Ok(worker)
    }

    fn wait_started(&self) -> Result<(), String> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| "startup lock poisoned")?;
        while !state.started && !state.stop {
            state = self
                .shared
                .done
                .wait(state)
                .map_err(|_| "startup wait poisoned")?;
        }
        if state.stop {
            Err("worker stopped during startup".into())
        } else {
            Ok(())
        }
    }

    fn submit(&self, operation: Operation) -> Result<(), WorkerFailure> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorkerFailure::Protocol)?;
        if state.stop || state.pending.is_some() || state.complete.is_some() {
            return Err(WorkerFailure::Protocol);
        }
        state.pending = Some(operation);
        drop(state);
        self.shared.ready.notify_one();
        Ok(())
    }

    fn collect(&self) -> Result<(), WorkerFailure> {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while state.complete.is_none() {
            state = self
                .shared
                .done
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.complete.take().ok_or(WorkerFailure::Protocol)?
    }

    fn publish(&self, operation: Operation) -> Result<(), WorkerFailure> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorkerFailure::Protocol)?;
        state.lane.publish(operation);
        Ok(())
    }

    fn reset(&self, variant: usize) -> Result<(), String> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| "reset lock poisoned")?;
        if state.pending.is_some() || state.complete.is_some() {
            return Err("reset of busy worker".into());
        }
        state.lane.reset(variant);
        Ok(())
    }

    fn inject_failure(&self, panic: bool) {
        let mut state = self.shared.state.lock().unwrap();
        if panic {
            state.panic_next = true;
        } else {
            state.fail_next = true;
        }
    }

    fn quiescent(&self) -> bool {
        let state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending.is_none() && state.complete.is_none()
    }

    fn copy_to(
        &self,
        operation: Operation,
        physical: &mut [f64],
        spectrum: &mut [Complex64],
    ) -> Result<(), String> {
        let state = self.shared.state.lock().map_err(|_| "copy lock poisoned")?;
        match operation {
            Operation::Forward => spectrum.copy_from_slice(&state.lane.spectrum),
            Operation::Inverse => physical.copy_from_slice(&state.lane.physical),
            Operation::Noop => {}
        }
        Ok(())
    }

    fn hashes(&self) -> Result<(String, String), String> {
        let state = self.shared.state.lock().map_err(|_| "hash lock poisoned")?;
        Ok((
            hash_real(&state.lane.physical),
            hash_complex(&state.lane.spectrum),
        ))
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.stop = true;
        drop(state);
        self.shared.ready.notify_one();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn worker_loop(shared: &Shared) {
    let mut state = shared.state.lock().expect("private startup lock");
    state.started = true;
    drop(state);
    shared.done.notify_one();
    loop {
        let mut state = shared.state.lock().expect("private worker lock");
        while state.pending.is_none() && !state.stop {
            state = shared.ready.wait(state).expect("private worker wait");
        }
        if state.stop {
            return;
        }
        let operation = state.pending.take().expect("admitted request");
        let fail = state.fail_next;
        let panic = state.panic_next;
        state.fail_next = false;
        state.panic_next = false;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if panic {
                panic!("injected worker panic");
            }
            if fail {
                return Err(WorkerFailure::Injected);
            }
            state
                .lane
                .run(operation)
                .map_err(|_| WorkerFailure::Numerical)
        }))
        .unwrap_or(Err(WorkerFailure::Panic));
        state.complete = Some(result);
        drop(state);
        shared.done.notify_one();
    }
}

struct ParallelBatch {
    workers: Vec<Worker>,
    failed: bool,
    copy_physical: Vec<Vec<f64>>,
    copy_spectrum: Vec<Vec<Complex64>>,
}

impl ParallelBatch {
    fn new(layout: Layout, catalog: &FftCatalog, cap: usize) -> Result<Self, String> {
        let reservation = reservation(layout)?;
        if reservation.parallel_profile_bytes > cap {
            return Err("resource-limit".into());
        }
        let mut workers = Vec::new();
        workers
            .try_reserve_exact(WIDTH)
            .map_err(|_| "worker allocation")?;
        for _ in 0..WIDTH {
            workers.push(Worker::new(Lane::new(layout, catalog)?)?);
        }
        let mut copy_physical = Vec::new();
        let mut copy_spectrum = Vec::new();
        copy_physical
            .try_reserve_exact(WIDTH)
            .map_err(|_| "copy physical owner")?;
        copy_spectrum
            .try_reserve_exact(WIDTH)
            .map_err(|_| "copy spectral owner")?;
        for _ in 0..WIDTH {
            copy_physical.push(filled(layout.real_len(), 0.0)?);
            copy_spectrum.push(filled(layout.half_len(), Complex64::new(0.0, 0.0))?);
        }
        Ok(Self {
            workers,
            failed: false,
            copy_physical,
            copy_spectrum,
        })
    }

    fn reset(&self, base: usize) -> Result<(), String> {
        for (worker, value) in self.workers.iter().zip(base..) {
            worker.reset(value)?;
        }
        Ok(())
    }

    fn execute(&mut self, operation: Operation) -> Result<(), WorkerFailure> {
        if self.failed {
            return Err(WorkerFailure::Protocol);
        }
        let mut submitted = 0;
        let mut failure = None;
        for worker in &self.workers {
            match worker.submit(operation) {
                Ok(()) => submitted += 1,
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        for worker in &self.workers[..submitted] {
            if let Err(error) = worker.collect() {
                failure.get_or_insert(error);
            }
        }
        if let Some(error) = failure {
            self.failed = true;
            return Err(error);
        }
        // Publication is deliberately after every completion and in ascending lane order.
        for worker in &self.workers {
            worker.publish(operation)?;
        }
        Ok(())
    }

    fn copy_published(&mut self, operation: Operation) -> Result<(), String> {
        for lane in 0..WIDTH {
            self.workers[lane].copy_to(
                operation,
                &mut self.copy_physical[lane],
                &mut self.copy_spectrum[lane],
            )?;
        }
        black_box((&self.copy_physical, &self.copy_spectrum));
        Ok(())
    }

    fn hashes(&self) -> Result<Vec<(String, String)>, String> {
        self.workers.iter().map(Worker::hashes).collect()
    }

    fn quiescent(&self) -> bool {
        self.workers.iter().all(Worker::quiescent)
    }
}

#[derive(Debug)]
struct Reservation {
    catalog_bytes: usize,
    scalar_workspace_bytes: usize,
    physical_lane_bytes: usize,
    spectral_lane_bytes: usize,
    serial_harness_bytes: usize,
    parallel_owner_bytes: usize,
    copy_adapter_bytes: usize,
    parallel_profile_bytes: usize,
    production_forward_additional_bytes: usize,
    production_inverse_additional_bytes: usize,
    production_bidirectional_additional_bytes: usize,
}

fn reservation(layout: Layout) -> Result<Reservation, String> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::reservation(backend).map_err(debug)?;
    let workspace = FftPlan::reservation_with_shared_backend(layout, backend).map_err(debug)?;
    let physical = layout
        .real_len()
        .checked_mul(8)
        .ok_or("physical overflow")?;
    let spectral = layout
        .half_len()
        .checked_mul(16)
        .ok_or("spectral overflow")?;
    let lane_vectors = physical.checked_add(spectral).ok_or("lane overflow")?;
    let lane_vector_allowances = 4 * ALLOCATION_ALLOWANCE;
    let copy_vector_allowances = 2 * ALLOCATION_ALLOWANCE;
    let serial = catalog
        .checked_add(WIDTH * workspace)
        .and_then(|n| n.checked_add(WIDTH * 2 * lane_vectors))
        .and_then(|n| {
            n.checked_add(
                size_of::<SerialBatch>()
                    + WIDTH * (size_of::<Lane>() + lane_vector_allowances)
                    + ALLOCATION_ALLOWANCE,
            )
        })
        .ok_or("serial reservation overflow")?;
    let worker_metadata = size_of::<Worker>()
        + size_of::<Shared>()
        + size_of::<WorkerState>()
        + 3 * ALLOCATION_ALLOWANCE;
    let parallel_owner = catalog
        .checked_add(WIDTH * workspace)
        .and_then(|n| n.checked_add(WIDTH * 2 * lane_vectors))
        .and_then(|n| {
            n.checked_add(
                WIDTH * (STACK_BYTES + THREAD_ALLOWANCE + worker_metadata + lane_vector_allowances),
            )
        })
        .and_then(|n| n.checked_add(size_of::<ParallelBatch>() + ALLOCATION_ALLOWANCE))
        .ok_or("parallel owner reservation overflow")?;
    let copy_adapter = WIDTH
        .checked_mul(lane_vectors + copy_vector_allowances)
        .and_then(|n| n.checked_add(2 * ALLOCATION_ALLOWANCE))
        .ok_or("copy adapter reservation overflow")?;
    let parallel_profile = parallel_owner
        .checked_add(copy_adapter)
        .ok_or("parallel profile reservation overflow")?;
    let worker_increment = WIDTH
        .checked_mul(STACK_BYTES + THREAD_ALLOWANCE + worker_metadata)
        .and_then(|n| n.checked_add(ALLOCATION_ALLOWANCE))
        .ok_or("worker increment overflow")?;
    let production_forward = 2usize
        .checked_mul(workspace)
        .and_then(|n| n.checked_add(2 * spectral))
        .and_then(|n| n.checked_add(2 * ALLOCATION_ALLOWANCE))
        .and_then(|n| n.checked_add(worker_increment))
        .ok_or("forward increment overflow")?;
    let production_inverse = 2usize
        .checked_mul(workspace)
        .and_then(|n| n.checked_add(2 * physical))
        .and_then(|n| n.checked_add(2 * ALLOCATION_ALLOWANCE))
        .and_then(|n| n.checked_add(worker_increment))
        .ok_or("inverse increment overflow")?;
    let production_bidirectional = 2usize
        .checked_mul(workspace)
        .and_then(|n| n.checked_add(2 * lane_vectors))
        .and_then(|n| n.checked_add(4 * ALLOCATION_ALLOWANCE))
        .and_then(|n| n.checked_add(worker_increment))
        .ok_or("bidirectional increment overflow")?;
    Ok(Reservation {
        catalog_bytes: catalog,
        scalar_workspace_bytes: workspace,
        physical_lane_bytes: physical,
        spectral_lane_bytes: spectral,
        serial_harness_bytes: serial,
        parallel_owner_bytes: parallel_owner,
        copy_adapter_bytes: copy_adapter,
        parallel_profile_bytes: parallel_profile,
        production_forward_additional_bytes: production_forward,
        production_inverse_additional_bytes: production_inverse,
        production_bidirectional_additional_bytes: production_bidirectional,
    })
}

fn profile(length: usize, catalog: &FftCatalog) -> Result<(), String> {
    let layout = Layout::new([length; 3]).map_err(debug)?;
    let bytes = reservation(layout)?;
    println!(
        "reservation n={length} catalog_bytes={} scalar_workspace_bytes={} physical_lane_bytes={} spectral_lane_bytes={} serial_harness_bytes={} parallel_owner_bytes={} copy_adapter_bytes={} parallel_profile_bytes={} production_forward_additional_bytes={} production_inverse_additional_bytes={} production_bidirectional_additional_bytes={} stack_bytes_each={STACK_BYTES} thread_allowance_each={THREAD_ALLOWANCE} allocation_allowance_each={ALLOCATION_ALLOWANCE}",
        bytes.catalog_bytes,
        bytes.scalar_workspace_bytes,
        bytes.physical_lane_bytes,
        bytes.spectral_lane_bytes,
        bytes.serial_harness_bytes,
        bytes.parallel_owner_bytes,
        bytes.copy_adapter_bytes,
        bytes.parallel_profile_bytes,
        bytes.production_forward_additional_bytes,
        bytes.production_inverse_additional_bytes,
        bytes.production_bidirectional_additional_bytes,
    );
    let refusal = match ParallelBatch::new(layout, catalog, bytes.parallel_profile_bytes - 1) {
        Ok(_) => return Err(format!("n={length} one-byte-short cap admitted")),
        Err(error) => error,
    };
    if refusal != "resource-limit" {
        return Err(format!("n={length} one-byte-short cap did not refuse"));
    }
    let mut serial = SerialBatch::new(layout, catalog)?;
    let mut parallel = ParallelBatch::new(layout, catalog, bytes.parallel_profile_bytes)?;
    serial.reset(100 + length);
    parallel.reset(100 + length)?;
    serial.execute(Operation::Forward)?;
    serial.execute(Operation::Inverse)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;

    let region = Region::new(GLOBAL);
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    let steady = region.change();
    if (
        steady.allocations,
        steady.deallocations,
        steady.reallocations,
    ) != (0, 0, 0)
    {
        return Err(format!("n={length} steady allocation: {steady:?}"));
    }

    let mut serial_forward = [Duration::ZERO; 3];
    let mut serial_inverse = [Duration::ZERO; 3];
    let mut parallel_forward = [Duration::ZERO; 3];
    let mut parallel_inverse = [Duration::ZERO; 3];
    for pair in 0..3 {
        if pair % 2 == 0 {
            (serial_forward[pair], serial_inverse[pair]) = timed_serial_cycle(&mut serial)?;
            (parallel_forward[pair], parallel_inverse[pair]) = timed_parallel_cycle(&mut parallel)?;
        } else {
            (parallel_forward[pair], parallel_inverse[pair]) = timed_parallel_cycle(&mut parallel)?;
            (serial_forward[pair], serial_inverse[pair]) = timed_serial_cycle(&mut serial)?;
        }
        println!(
            "pair n={length} index={pair} serial_forward_seconds={:.9} w3_forward_seconds={:.9} serial_inverse_seconds={:.9} w3_inverse_seconds={:.9}",
            serial_forward[pair].as_secs_f64(),
            parallel_forward[pair].as_secs_f64(),
            serial_inverse[pair].as_secs_f64(),
            parallel_inverse[pair].as_secs_f64(),
        );
    }
    serial_forward.sort();
    serial_inverse.sort();
    parallel_forward.sort();
    parallel_inverse.sort();

    let dispatch_repeats = 101;
    let dispatch_started = Instant::now();
    for _ in 0..dispatch_repeats {
        parallel.execute(Operation::Noop).map_err(worker_debug)?;
    }
    let dispatch = dispatch_started.elapsed().div_f64(dispatch_repeats as f64);
    let copy_repeats = if length <= 384 { 3 } else { 1 };
    let copy_forward = timed_copy(&mut parallel, Operation::Forward, copy_repeats)?;
    let copy_inverse = timed_copy(&mut parallel, Operation::Inverse, copy_repeats)?;

    serial.reset(700 + length);
    parallel.reset(700 + length)?;
    serial.execute(Operation::Forward)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    compare_serial_parallel(&serial, &parallel, Operation::Forward)?;
    serial.execute(Operation::Inverse)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    compare_serial_parallel(&serial, &parallel, Operation::Inverse)?;
    let first_hash = parallel.hashes()?;
    parallel.reset(700 + length)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    parallel.execute(Operation::Inverse).map_err(worker_debug)?;
    let second_hash = parallel.hashes()?;
    if first_hash != second_hash {
        return Err(format!("n={length} repeat hashes changed"));
    }

    let serial_cycle = serial_forward[1] + serial_inverse[1];
    let w3_cycle = parallel_forward[1] + parallel_inverse[1];
    let w3_copy_cycle = w3_cycle + copy_forward + copy_inverse;
    println!(
        "summary n={length} scalar_transforms_per_direction=3 serial_forward_median_seconds={:.9} w3_forward_median_seconds={:.9} forward_speedup={:.6} serial_inverse_median_seconds={:.9} w3_inverse_median_seconds={:.9} inverse_speedup={:.6} serial_cycle_seconds={:.9} w3_cycle_seconds={:.9} swap_publish_speedup={:.6} copy_forward_seconds={:.9} copy_inverse_seconds={:.9} copy_inclusive_w3_cycle_seconds={:.9} copy_inclusive_speedup={:.6} dispatch_seconds={:.9} steady_allocations={} output_equal_words=true repeat_deterministic=true output_sha256={}",
        serial_forward[1].as_secs_f64(),
        parallel_forward[1].as_secs_f64(),
        serial_forward[1].as_secs_f64() / parallel_forward[1].as_secs_f64(),
        serial_inverse[1].as_secs_f64(),
        parallel_inverse[1].as_secs_f64(),
        serial_inverse[1].as_secs_f64() / parallel_inverse[1].as_secs_f64(),
        serial_cycle.as_secs_f64(),
        w3_cycle.as_secs_f64(),
        serial_cycle.as_secs_f64() / w3_cycle.as_secs_f64(),
        copy_forward.as_secs_f64(),
        copy_inverse.as_secs_f64(),
        w3_copy_cycle.as_secs_f64(),
        serial_cycle.as_secs_f64() / w3_copy_cycle.as_secs_f64(),
        dispatch.as_secs_f64(),
        steady.allocations,
        first_hash[0].0,
    );
    Ok(())
}

fn timed_serial_cycle(batch: &mut SerialBatch) -> Result<(Duration, Duration), String> {
    let started = Instant::now();
    batch.execute(Operation::Forward)?;
    let forward = started.elapsed();
    let started = Instant::now();
    batch.execute(Operation::Inverse)?;
    Ok((forward, started.elapsed()))
}

fn timed_parallel_cycle(batch: &mut ParallelBatch) -> Result<(Duration, Duration), String> {
    let started = Instant::now();
    batch.execute(Operation::Forward).map_err(worker_debug)?;
    let forward = started.elapsed();
    let started = Instant::now();
    batch.execute(Operation::Inverse).map_err(worker_debug)?;
    Ok((forward, started.elapsed()))
}

fn timed_copy(
    batch: &mut ParallelBatch,
    operation: Operation,
    repeats: usize,
) -> Result<Duration, String> {
    let started = Instant::now();
    for _ in 0..repeats {
        batch.copy_published(operation)?;
    }
    Ok(started.elapsed().div_f64(repeats as f64))
}

fn compare_serial_parallel(
    serial: &SerialBatch,
    parallel: &ParallelBatch,
    operation: Operation,
) -> Result<(), String> {
    for lane in 0..WIDTH {
        let state = parallel.workers[lane]
            .shared
            .state
            .lock()
            .map_err(|_| "comparison lock poisoned")?;
        let equal = match operation {
            Operation::Forward => serial.lanes[lane].spectrum == state.lane.spectrum,
            Operation::Inverse => serial.lanes[lane].physical == state.lane.physical,
            Operation::Noop => true,
        };
        if !equal {
            return Err(format!("lane {lane} {operation:?} differs from serial"));
        }
    }
    Ok(())
}

fn direct_reference_and_failure_controls(catalog: &FftCatalog) -> Result<(), String> {
    let layout = Layout::new([6; 3]).map_err(debug)?;
    let mut serial = SerialBatch::new(layout, catalog)?;
    let cap = reservation(layout)?.parallel_profile_bytes;
    let mut parallel = ParallelBatch::new(layout, catalog, cap)?;
    serial.reset(17);
    parallel.reset(17)?;
    let direct = direct_dft(layout, &serial.lanes[0].physical);
    serial.execute(Operation::Forward)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    compare_serial_parallel(&serial, &parallel, Operation::Forward)?;
    let error = maximum_scaled(&direct, &serial.lanes[0].spectrum);
    if error > 2e-14 {
        return Err(format!("direct reference error {error}"));
    }

    let prior = parallel.hashes()?;
    parallel.workers[1].inject_failure(false);
    if parallel.execute(Operation::Inverse) != Err(WorkerFailure::Injected)
        || !parallel.quiescent()
        || parallel.hashes()? != prior
    {
        return Err("injected worker failure did not drain/preserve publication".into());
    }
    let mut panic_parallel = ParallelBatch::new(layout, catalog, cap)?;
    panic_parallel.reset(31)?;
    let prior = panic_parallel.hashes()?;
    panic_parallel.workers[2].inject_failure(true);
    if panic_parallel.execute(Operation::Forward) != Err(WorkerFailure::Panic)
        || !panic_parallel.quiescent()
        || panic_parallel.hashes()? != prior
    {
        return Err("injected worker panic did not drain/preserve publication".into());
    }
    println!(
        "controls direct_dft_n=6 maximum_scaled={error:.17e} serial_w3_equal_words=true injected_error_drained=true injected_panic_drained=true failed_publication_preserved=true one_byte_short_caps_checked=true"
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

fn maximum_scaled(reference: &[Complex64], actual: &[Complex64]) -> f64 {
    reference
        .iter()
        .zip(actual)
        .map(|(a, b)| (*a - *b).norm() / a.norm().max(1.0))
        .fold(0.0_f64, f64::max)
}

fn fill_input(values: &mut [f64], variant: usize) {
    let mut bits = (variant as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    for (index, value) in values.iter_mut().enumerate() {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let fraction = ((bits >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64));
        *value = 2.0 * fraction - 1.0 + (index % 17) as f64 * 1e-5;
    }
}

fn filled<T: Clone>(length: usize, value: T) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "allocation failed")?;
    values.resize(length, value);
    Ok(values)
}

fn hash_real(values: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn hash_complex(values: &[Complex64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn worker_debug(error: WorkerFailure) -> String {
    format!("{error:?}")
}
