//! Opt-in deterministic three-component FFT execution with owned worker state.
mod admission;
mod worker;
use super::{FftBackend, FftCatalog, FftPlan, FftWorkspace};
use crate::{domain::Layout, storage::filled, Complex64, SolverError};
use worker::{Lane, Operation, Worker};

const WIDTH: usize = 3;
const STACK_BYTES: usize = 2 * 1024 * 1024;

/// Storage directions admitted by one experimental W3 owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W3FftMode {
    /// Three-component forward transforms only.
    Forward,
    /// Three-component forward and inverse transforms.
    Bidirectional,
}

/// Complete execution identity for one opt-in W3 owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W3FftIdentity {
    /// Transform layout.
    pub layout: Layout,
    /// Scalar arithmetic/backend identity.
    pub backend: FftBackend,
    /// Fixed component width.
    pub width: usize,
    /// Admitted directions.
    pub mode: W3FftMode,
    /// Additional bytes beyond the consumed scalar owner.
    pub additional_bytes: usize,
}

/// Three persistent workers consuming one existing scalar plan/workspace lane.
///
/// Transform/worker failures, including worker panics, drain and terminate; scratch is unspecified
/// and owners publish on success. Caller callbacks execute under private lane locks and must not
/// panic or reenter this owner; those caller failures are outside the worker-panic guarantee.
pub struct W3FftPool {
    workers: Vec<Worker>,
    identity: W3FftIdentity,
    failed: bool,
}

impl W3FftPool {
    /// Exact conservative addition beyond one scalar workspace and spectral lane.
    pub fn additional_reservation(
        layout: Layout,
        catalog: &FftCatalog,
        mode: W3FftMode,
    ) -> Result<usize, SolverError> {
        Self::additional_reservation_with_backend(layout, catalog.backend(), mode)
    }

    /// Preflight the same addition before constructing the immutable catalog.
    pub fn additional_reservation_with_backend(
        layout: Layout,
        backend: FftBackend,
        mode: W3FftMode,
    ) -> Result<usize, SolverError> {
        admission::additional(layout, backend, mode)
    }

    /// Consume one scalar lane and construct the two extra lanes before starting workers.
    pub fn from_scalar_lane(
        layout: Layout,
        catalog: &FftCatalog,
        mode: W3FftMode,
        seed: (FftPlan, FftWorkspace, Vec<Complex64>),
        cap: usize,
    ) -> Result<Self, SolverError> {
        if !seed.0.matches_lane(layout, catalog.backend(), &seed.1)
            || seed.2.len() != layout.half_len()
            || seed.2.capacity() < layout.half_len()
        {
            return Err(SolverError::InvalidPayload);
        }
        let additional = Self::additional_reservation(layout, catalog, mode)?;
        if additional > cap {
            return Err(SolverError::ResourceLimit);
        }
        let mut lanes = Vec::new();
        lanes
            .try_reserve_exact(WIDTH)
            .map_err(|_| SolverError::AllocationFailed)?;
        lanes.push(Lane {
            plan: seed.0,
            workspace: seed.1,
            physical: Vec::new(),
            spectrum: seed.2,
        });
        for _ in 1..WIDTH {
            let bytes = FftPlan::reservation_from_catalog(layout, catalog)?;
            let (plan, workspace) = FftPlan::new_from_catalog(layout, catalog, bytes)?;
            let physical = match mode {
                W3FftMode::Forward => Vec::new(),
                W3FftMode::Bidirectional => filled(layout.real_len(), 0.0)?,
            };
            lanes.push(Lane {
                plan,
                workspace,
                physical,
                spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
            });
        }
        let mut workers = Vec::new();
        workers
            .try_reserve_exact(WIDTH)
            .map_err(|_| SolverError::AllocationFailed)?;
        for lane in lanes {
            workers.push(Worker::new(lane)?);
        }
        Ok(Self {
            workers,
            identity: W3FftIdentity {
                layout,
                backend: catalog.backend(),
                width: WIDTH,
                mode,
                additional_bytes: additional,
            },
            failed: false,
        })
    }

    /// Bound layout, width, direction support and additional reservation.
    pub fn identity(&self) -> W3FftIdentity {
        self.identity
    }

    /// Whether a drained numerical failure or panic permanently terminated this owner.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }

    #[cfg(test)]
    pub(super) fn inject_failure(&self, lane: usize, panic: bool) {
        let mut state = self.workers[lane].shared.state.lock().unwrap();
        if panic {
            state.panic_next = true;
        } else {
            state.fail_next = true;
        }
    }

    #[cfg(test)]
    pub(super) fn all_collected(&self) -> bool {
        self.workers.iter().all(|worker| {
            let state = worker
                .shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.pending.is_none() && state.complete.is_none()
        })
    }

    /// Fill one worker-owned inverse input while the owner is quiescent.
    /// The callback executes under a private lane lock and must not panic or reenter this owner.
    pub fn prepare_inverse(
        &self,
        lane: usize,
        fill: impl FnOnce(&mut [Complex64]),
    ) -> Result<(), SolverError> {
        if self.failed {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if self.identity.mode != W3FftMode::Bidirectional || lane >= WIDTH {
            return Err(SolverError::InvalidPayload);
        }
        let mut state = self.workers[lane]
            .shared
            .state
            .lock()
            .map_err(|_| SolverError::InvalidPayload)?;
        if state.pending.is_some() || state.complete.is_some() {
            return Err(SolverError::InvalidPayload);
        }
        fill(&mut state.lane.spectrum);
        Ok(())
    }

    /// Execute three inverse transforms and drain all lanes before returning outputs.
    pub fn inverse3(&mut self, outputs: &mut [Vec<f64>; WIDTH]) -> Result<(), SolverError> {
        if self.failed || self.identity.mode != W3FftMode::Bidirectional {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if outputs
            .iter()
            .any(|v| v.len() != self.identity.layout.real_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        for (worker, output) in self.workers.iter().zip(outputs.iter_mut()) {
            let mut state = worker
                .shared
                .state
                .lock()
                .map_err(|_| SolverError::InvalidPayload)?;
            std::mem::swap(&mut state.lane.physical, output);
        }
        let result = self.execute(Operation::Inverse);
        for (worker, output) in self.workers.iter().zip(outputs.iter_mut()) {
            let mut state = worker
                .shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::swap(&mut state.lane.physical, output);
        }
        result
    }

    /// Execute three forward transforms while worker lanes own all submitted Vecs.
    pub fn forward3(&mut self, inputs: &mut [Vec<f64>; WIDTH]) -> Result<(), SolverError> {
        if self.failed {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if inputs
            .iter()
            .any(|v| v.len() != self.identity.layout.real_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        for (worker, input) in self.workers.iter().zip(inputs.iter_mut()) {
            let mut state = worker
                .shared
                .state
                .lock()
                .map_err(|_| SolverError::InvalidPayload)?;
            std::mem::swap(&mut state.lane.physical, input);
        }
        let result = self.execute(Operation::Forward);
        for (worker, input) in self.workers.iter().zip(inputs.iter_mut()) {
            let mut state = worker
                .shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::swap(&mut state.lane.physical, input);
        }
        result
    }

    /// Preserve a scalar transform for the pressure path without batching it.
    pub fn forward_one(&mut self, input: &[f64]) -> Result<(), SolverError> {
        if self.failed {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if input.len() != self.identity.layout.real_len() {
            return Err(SolverError::InvalidPayload);
        }
        let result = {
            let mut state = self.workers[0]
                .shared
                .state
                .lock()
                .map_err(|_| SolverError::InvalidPayload)?;
            let lane = &mut state.lane;
            lane.plan
                .forward(input, &mut lane.spectrum, &mut lane.workspace)
        };
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Read one completed full spectrum without exposing worker ownership.
    /// The callback executes under a private lane lock and must not panic or reenter this owner.
    pub fn with_spectrum<T>(
        &self,
        lane: usize,
        read: impl FnOnce(&[Complex64]) -> T,
    ) -> Result<T, SolverError> {
        if self.failed || lane >= WIDTH {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        let state = self.workers[lane]
            .shared
            .state
            .lock()
            .map_err(|_| SolverError::InvalidPayload)?;
        Ok(read(&state.lane.spectrum))
    }

    fn execute(&mut self, operation: Operation) -> Result<(), SolverError> {
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
        if failure.is_some() {
            self.failed = true;
            Err(SolverError::ArithmeticResolutionLimited)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
