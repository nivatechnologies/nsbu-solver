//! Shared bounded nonautonomous source for transactional and payload-copy checks.
use nsbu_solver::{
    domain::TickClock,
    integrators::kernel::{RhsBounds, RightHandSide},
    Complex64, SolverError,
};

pub struct Source {
    calls: usize,
    fail_at: usize,
    amplitudes: [[Complex64; 2]; 3],
}
impl Source {
    pub fn new(fail_at: usize, amplitudes: [[Complex64; 2]; 3]) -> Self {
        Self {
            calls: 0,
            fail_at,
            amplitudes,
        }
    }
}
impl RightHandSide for Source {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.calls += 1;
        if self.calls == self.fail_at {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        let value = (clock.elapsed() as f64).cos();
        for (component, amplitude) in output.into_iter().zip(self.amplitudes) {
            component.fill(Complex64::new(0.0, 0.0));
            component[0] = amplitude[0] * value;
            component[1] = amplitude[1] * value;
        }
        Ok(())
    }
}
