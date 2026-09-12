//! Persistent workers own fixed sample buffers; no threads are created during a force request.
use super::sampling::{Arithmetic, Partition, Samples};
use crate::time::BenchmarkTime;
use nsbu_solver::SolverError;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{Builder, JoinHandle};
pub(super) const STACK_BYTES: usize = 2 * 1024 * 1024;
pub(super) const THREAD_ALLOWANCE: usize = 64 * 1024;
struct State {
    started: bool,
    pending: Option<BenchmarkTime>,
    complete: Option<Result<usize, SolverError>>,
    stop: bool,
    samples: Samples,
    #[cfg(test)]
    fail_next: bool,
    #[cfg(test)]
    panic_next: bool,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
    done: Condvar,
}
pub(super) struct Worker {
    partition: Partition,
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn metadata_bytes() -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of::<Shared>()
            + 2 * std::mem::size_of::<usize>()
    }
    pub fn new(partition: Partition, arithmetic: Arithmetic) -> Result<Self, SolverError> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                started: false,
                pending: None,
                complete: None,
                stop: false,
                samples: Samples::new(partition, arithmetic)?,
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
            .spawn(move || {
                // An unexpected programming panic must notify the waiting owner, never strand it.
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(&thread, partition)
                }))
                .is_err()
                {
                    let mut state = thread
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.stop = true;
                    state.complete = Some(Err(SolverError::ArithmeticResolutionLimited));
                    drop(state);
                    thread.done.notify_all();
                }
            })
            .map_err(|_| SolverError::AllocationFailed)?;
        let result = Self {
            partition,
            shared,
            handle: Some(handle),
        };
        result.wait_started()?;
        Ok(result)
    }
    fn wait_started(&self) -> Result<(), SolverError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        while !state.started && !state.stop {
            state = self
                .shared
                .done
                .wait(state)
                .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        }
        if state.stop {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        Ok(())
    }
    pub fn submit(&self, time: BenchmarkTime) -> Result<(), SolverError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        if state.stop || state.pending.is_some() || state.complete.is_some() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        state.pending = Some(time);
        drop(state);
        self.shared.ready.notify_one();
        Ok(())
    }
    pub fn collect(&self, output: &mut [Vec<f64>; 3]) -> Result<usize, SolverError> {
        // The outer panic handler still publishes a completion after an unwound lock.
        // Drain that completion before returning; poisoned sample data is never copied.
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
        let mut result = state.complete.take().ok_or(SolverError::InvalidPayload)?;
        if self.shared.state.is_poisoned() {
            result = Err(SolverError::ArithmeticResolutionLimited);
        }
        if result.is_ok() {
            state.samples.copy_to(self.partition, output);
        }
        result
    }
    #[cfg(test)]
    pub fn quiescent(&self) -> bool {
        let state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending.is_none() && state.complete.is_none()
    }
    #[cfg(test)]
    pub fn fail_next(&self) {
        self.shared.state.lock().unwrap().fail_next = true;
    }
    #[cfg(test)]
    pub fn panic_next(&self) {
        self.shared.state.lock().unwrap().panic_next = true;
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
fn run(shared: &Shared, partition: Partition) {
    let mut state = shared.state.lock().expect("private worker startup");
    state.started = true;
    drop(state);
    shared.done.notify_one();
    loop {
        let mut state = shared.state.lock().expect("private worker state lock");
        while state.pending.is_none() && !state.stop {
            state = shared.ready.wait(state).expect("private worker condition");
        }
        if state.stop {
            return;
        }
        let time = state
            .pending
            .take()
            .expect("admitted pending worker request");
        state.complete = Some(evaluate(&mut state, partition, time));
        drop(state);
        shared.done.notify_one();
    }
}
fn evaluate(
    state: &mut State,
    partition: Partition,
    time: BenchmarkTime,
) -> Result<usize, SolverError> {
    #[cfg(test)]
    if state.panic_next {
        panic!("injected private worker panic");
    }
    #[cfg(test)]
    if state.fail_next {
        state.fail_next = false;
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    state.samples.evaluate(partition, time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::domain::{Layout, TickClock};
    #[test]
    fn dropping_idle_and_submitted_workers_joins_and_releases_all_shared_storage() {
        for arithmetic in [Arithmetic::Cartesian, Arithmetic::Reduced] {
            for pending in [false, true] {
                let worker = Worker::new(
                    Partition::new(Layout::new([4; 3]).unwrap(), 0, 1),
                    arithmetic,
                )
                .unwrap();
                let weak = Arc::downgrade(&worker.shared);
                if pending {
                    worker
                        .submit(
                            BenchmarkTime::new(TickClock::restore(-10, 8, 1, 7).unwrap()).unwrap(),
                        )
                        .unwrap();
                }
                drop(worker);
                assert!(weak.upgrade().is_none());
            }
        }
    }
}
