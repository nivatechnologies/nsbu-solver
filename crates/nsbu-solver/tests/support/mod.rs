//! Shared exact-clock call recorder for test-only RHS probes.
use nsbu_solver::{domain::TickClock, SolverError};

pub struct CallLog {
    pub ticks: Vec<u128>,
    pub fail_at: usize,
}
impl CallLog {
    pub fn record(&mut self, time: TickClock) -> Result<(), SolverError> {
        self.ticks.push(time.elapsed());
        if self.ticks.len() == self.fail_at {
            return Err(SolverError::ResourceLimit);
        }
        Ok(())
    }
}
