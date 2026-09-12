//! Complete dispatch/collection boundary: every submitted worker is drained before returning.
use super::{
    sampling::{Arithmetic, Partition},
    worker::Worker,
};
use crate::time::BenchmarkTime;
use nsbu_solver::{domain::Layout, SolverError};
pub(in crate::provider) struct Pool {
    workers: Vec<Worker>,
    failed: bool,
}
impl Pool {
    pub fn new(
        layout: Layout,
        workers: usize,
        arithmetic: Arithmetic,
    ) -> Result<Self, SolverError> {
        let mut result = Self {
            workers: Vec::new(),
            failed: false,
        };
        result
            .workers
            .try_reserve_exact(workers)
            .map_err(|_| SolverError::AllocationFailed)?;
        for worker in 0..workers {
            result.workers.push(Worker::new(
                Partition::new(layout, worker, workers),
                arithmetic,
            )?);
        }
        Ok(result)
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub fn execute(
        &mut self,
        time: BenchmarkTime,
        output: &mut [Vec<f64>; 3],
    ) -> Result<usize, SolverError> {
        if self.failed {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        let mut submitted = 0;
        let mut error = None;
        for worker in &self.workers {
            match worker.submit(time) {
                Ok(()) => submitted += 1,
                Err(failure) => {
                    error = Some(failure);
                    break;
                }
            }
        }
        let mut iterations = 0;
        for worker in &self.workers[..submitted] {
            match worker.collect(output) {
                Ok(count) => iterations += count,
                Err(failure) => {
                    error.get_or_insert(failure);
                }
            }
        }
        match error {
            None => Ok(iterations),
            Some(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    #[cfg(test)]
    pub fn all_collected(&self) -> bool {
        self.workers.iter().all(Worker::quiescent)
    }
    #[cfg(test)]
    pub fn fail_next(&self, worker: usize) {
        self.workers[worker].fail_next();
    }
    #[cfg(test)]
    pub fn panic_next(&self, worker: usize) {
        self.workers[worker].panic_next();
    }
}
