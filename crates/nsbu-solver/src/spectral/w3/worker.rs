//! Persistent worker lane protocol and panic-to-failure boundary.
use super::STACK_BYTES;
use crate::spectral::{FftPlan, FftWorkspace};
use crate::{Complex64, SolverError};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{Builder, JoinHandle},
};

#[derive(Clone, Copy)]
pub(super) enum Operation {
    Forward,
    Inverse,
}

#[derive(Clone, Copy)]
pub(super) enum Failure {
    Numerical,
    Panic,
    Protocol,
}

pub(super) struct Lane {
    pub(super) plan: FftPlan,
    pub(super) workspace: FftWorkspace,
    pub(super) physical: Vec<f64>,
    pub(super) spectrum: Vec<Complex64>,
}

pub(super) struct WorkerState {
    started: bool,
    pub(super) pending: Option<Operation>,
    pub(super) complete: Option<Result<(), Failure>>,
    stop: bool,
    pub(super) lane: Lane,
    #[cfg(test)]
    pub(super) fail_next: bool,
    #[cfg(test)]
    pub(super) panic_next: bool,
}

pub(super) struct Shared {
    pub(super) state: Mutex<WorkerState>,
    ready: Condvar,
    done: Condvar,
}

pub(super) struct Worker {
    pub(super) shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub(super) fn new(lane: Lane) -> Result<Self, SolverError> {
        let shared = Arc::new(Shared {
            state: Mutex::new(WorkerState {
                started: false,
                pending: None,
                complete: None,
                stop: false,
                lane,
                #[cfg(test)]
                fail_next: false,
                #[cfg(test)]
                panic_next: false,
            }),
            ready: Condvar::new(),
            done: Condvar::new(),
        });
        let thread = Arc::clone(&shared);
        let handle = Builder::new()
            .stack_size(STACK_BYTES)
            .spawn(move || worker_loop(&thread))
            .map_err(|_| SolverError::AllocationFailed)?;
        let worker = Self {
            shared,
            handle: Some(handle),
        };
        worker.wait_started()?;
        Ok(worker)
    }

    fn wait_started(&self) -> Result<(), SolverError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| SolverError::InvalidPayload)?;
        while !state.started && !state.stop {
            state = self
                .shared
                .done
                .wait(state)
                .map_err(|_| SolverError::InvalidPayload)?;
        }
        if state.stop {
            Err(SolverError::ArithmeticResolutionLimited)
        } else {
            Ok(())
        }
    }

    pub(super) fn submit(&self, operation: Operation) -> Result<(), Failure> {
        let mut state = self.shared.state.lock().map_err(|_| Failure::Protocol)?;
        if state.stop || state.pending.is_some() || state.complete.is_some() {
            return Err(Failure::Protocol);
        }
        state.pending = Some(operation);
        drop(state);
        self.shared.ready.notify_one();
        Ok(())
    }

    pub(super) fn collect(&self) -> Result<(), Failure> {
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
        state.complete.take().ok_or(Failure::Protocol)?
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
    let mut state = shared.state.lock().expect("private W3 startup lock");
    state.started = true;
    drop(state);
    shared.done.notify_one();
    loop {
        let mut state = shared.state.lock().expect("private W3 worker lock");
        while state.pending.is_none() && !state.stop {
            state = shared.ready.wait(state).expect("private W3 wait");
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
    let operation = state.pending.take().expect("admitted W3 request");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        #[cfg(test)]
        {
            if std::mem::take(&mut state.panic_next) {
                panic!("injected W3 panic");
            }
            if std::mem::take(&mut state.fail_next) {
                return Err(Failure::Numerical);
            }
        }
        let lane = &mut state.lane;
        match operation {
            Operation::Forward => {
                lane.plan
                    .forward(&lane.physical, &mut lane.spectrum, &mut lane.workspace)
            }
            Operation::Inverse => {
                lane.plan
                    .inverse(&lane.spectrum, &mut lane.physical, &mut lane.workspace)
            }
        }
        .map_err(|_| Failure::Numerical)
    }))
    .unwrap_or(Err(Failure::Panic));
    state.complete = Some(result);
}
