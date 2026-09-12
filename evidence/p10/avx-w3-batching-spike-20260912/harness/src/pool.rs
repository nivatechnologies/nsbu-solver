use crate::{
    model::{Lane, Operation, SerialBatch},
    util::{
        filled, hash_complex, hash_real, ALLOCATION_ALLOWANCE, STACK_BYTES, THREAD_ALLOWANCE, WIDTH,
    },
};
use nsbu_solver::{
    domain::Layout,
    spectral::{FftBackend, FftCatalog, FftPlan},
    Complex64,
};
use std::{
    hint::black_box,
    mem::size_of,
    sync::{Arc, Condvar, Mutex},
    thread::{Builder, JoinHandle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkerFailure {
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

    fn published_equals(&self, reference: &Lane, operation: Operation) -> Result<bool, String> {
        let state = self
            .shared
            .state
            .lock()
            .map_err(|_| "comparison lock poisoned")?;
        Ok(match operation {
            Operation::Forward => reference.spectrum == state.lane.spectrum,
            Operation::Inverse => reference.physical == state.lane.physical,
            Operation::Noop => true,
        })
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
        evaluate_pending(&mut state);
        drop(state);
        shared.done.notify_one();
    }
}

fn evaluate_pending(state: &mut WorkerState) {
    let operation = state.pending.take().expect("admitted request");
    let fail = std::mem::take(&mut state.fail_next);
    let panic = std::mem::take(&mut state.panic_next);
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
}

pub(crate) struct ParallelBatch {
    workers: Vec<Worker>,
    failed: bool,
    copy_physical: Vec<Vec<f64>>,
    copy_spectrum: Vec<Vec<Complex64>>,
}

impl ParallelBatch {
    pub(crate) fn new(layout: Layout, catalog: &FftCatalog, cap: usize) -> Result<Self, String> {
        if reservation(layout)?.parallel_profile_bytes > cap {
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

    pub(crate) fn reset(&self, base: usize) -> Result<(), String> {
        for (worker, value) in self.workers.iter().zip(base..) {
            worker.reset(value)?;
        }
        Ok(())
    }

    pub(crate) fn execute(&mut self, operation: Operation) -> Result<(), WorkerFailure> {
        if self.failed {
            return Err(WorkerFailure::Protocol);
        }
        let (submitted, mut failure) = self.submit_all(operation);
        for worker in &self.workers[..submitted] {
            if let Err(error) = worker.collect() {
                failure.get_or_insert(error);
            }
        }
        if let Some(error) = failure {
            self.failed = true;
            return Err(error);
        }
        for worker in &self.workers {
            worker.publish(operation)?;
        }
        Ok(())
    }

    fn submit_all(&self, operation: Operation) -> (usize, Option<WorkerFailure>) {
        let mut submitted = 0;
        for worker in &self.workers {
            if let Err(error) = worker.submit(operation) {
                return (submitted, Some(error));
            }
            submitted += 1;
        }
        (submitted, None)
    }

    pub(crate) fn copy_published(&mut self, operation: Operation) -> Result<(), String> {
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

    pub(crate) fn hashes(&self) -> Result<Vec<(String, String)>, String> {
        self.workers.iter().map(Worker::hashes).collect()
    }

    pub(crate) fn quiescent(&self) -> bool {
        self.workers.iter().all(Worker::quiescent)
    }

    pub(crate) fn inject_failure(&self, lane: usize, panic: bool) {
        self.workers[lane].inject_failure(panic);
    }

    pub(crate) fn equals(
        &self,
        serial: &SerialBatch,
        operation: Operation,
    ) -> Result<bool, String> {
        for (worker, lane) in self.workers.iter().zip(&serial.lanes) {
            if !worker.published_equals(lane, operation)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Debug)]
pub(crate) struct Reservation {
    pub(crate) catalog_bytes: usize,
    pub(crate) scalar_workspace_bytes: usize,
    pub(crate) physical_lane_bytes: usize,
    pub(crate) spectral_lane_bytes: usize,
    pub(crate) serial_harness_bytes: usize,
    pub(crate) parallel_owner_bytes: usize,
    pub(crate) copy_adapter_bytes: usize,
    pub(crate) parallel_profile_bytes: usize,
    pub(crate) production_forward_additional_bytes: usize,
    pub(crate) production_inverse_additional_bytes: usize,
    pub(crate) production_bidirectional_additional_bytes: usize,
}

pub(crate) fn reservation(layout: Layout) -> Result<Reservation, String> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::reservation(backend).map_err(crate::util::debug)?;
    let workspace =
        FftPlan::reservation_with_shared_backend(layout, backend).map_err(crate::util::debug)?;
    let physical = layout
        .real_len()
        .checked_mul(8)
        .ok_or("physical overflow")?;
    let spectral = layout
        .half_len()
        .checked_mul(16)
        .ok_or("spectral overflow")?;
    reservation_parts(catalog, workspace, physical, spectral)
}

fn reservation_parts(
    catalog: usize,
    workspace: usize,
    physical: usize,
    spectral: usize,
) -> Result<Reservation, String> {
    let lane_vectors = physical.checked_add(spectral).ok_or("lane overflow")?;
    let worker = WIDTH * worker_bytes();
    let serial = catalog
        + WIDTH * workspace
        + WIDTH * 2 * lane_vectors
        + size_of::<SerialBatch>()
        + WIDTH * (size_of::<Lane>() + 4 * ALLOCATION_ALLOWANCE)
        + ALLOCATION_ALLOWANCE;
    let parallel_owner = catalog
        + WIDTH * workspace
        + WIDTH * 2 * lane_vectors
        + worker
        + size_of::<ParallelBatch>()
        + ALLOCATION_ALLOWANCE;
    let copy_adapter = WIDTH * (lane_vectors + 2 * ALLOCATION_ALLOWANCE) + 2 * ALLOCATION_ALLOWANCE;
    let worker_increment = worker + ALLOCATION_ALLOWANCE;
    Ok(Reservation {
        catalog_bytes: catalog,
        scalar_workspace_bytes: workspace,
        physical_lane_bytes: physical,
        spectral_lane_bytes: spectral,
        serial_harness_bytes: serial,
        parallel_owner_bytes: parallel_owner,
        copy_adapter_bytes: copy_adapter,
        parallel_profile_bytes: parallel_owner + copy_adapter,
        production_forward_additional_bytes: 2 * workspace
            + 2 * spectral
            + 2 * ALLOCATION_ALLOWANCE
            + worker_increment,
        production_inverse_additional_bytes: 2 * workspace
            + 2 * physical
            + 2 * ALLOCATION_ALLOWANCE
            + worker_increment,
        production_bidirectional_additional_bytes: 2 * workspace
            + 2 * lane_vectors
            + 4 * ALLOCATION_ALLOWANCE
            + worker_increment,
    })
}

fn worker_bytes() -> usize {
    STACK_BYTES
        + THREAD_ALLOWANCE
        + size_of::<Worker>()
        + size_of::<Shared>()
        + size_of::<WorkerState>()
        + 7 * ALLOCATION_ALLOWANCE
}
