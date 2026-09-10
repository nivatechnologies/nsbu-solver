//! Complete physical fields from actual accepted-history interpolants at exact probe times.
mod plan;
mod report;
#[cfg(test)]
mod tests;
use super::{ProbeFamily, ProbeSample};
use crate::smooth_experiment::{
    physical::PhysicalFamilyWorkspace, pressure::PressureFamilyWorkspace, FamilyError,
};
use nsbu_solver::{domain::TickClock, SolverError};
pub use plan::{ProbeDiagnosticBounds, ProbeDiagnosticWork, ProbeDiagnosticsPlan};
pub use report::ProbePhysicalSample;

/// Borrowed probe sampling with its own finite work ledger and complete-publication boundary.
/// It never mutates trajectories, assigns a reference or uses stage pressure/force values.
pub struct ProbeDiagnostics<'a> {
    plan: ProbeDiagnosticsPlan<'a>,
    physical: PhysicalFamilyWorkspace<'a>,
    pressure: PressureFamilyWorkspace<'a>,
    attempts: usize,
    next: usize,
    failed: bool,
}
impl<'a> ProbeDiagnostics<'a> {
    /// Allocate only the two consumers already covered by the complete joint admission.
    pub fn new(plan: ProbeDiagnosticsPlan<'a>) -> Result<Self, SolverError> {
        Ok(Self {
            plan,
            physical: PhysicalFamilyWorkspace::new(plan.physical)?,
            pressure: PressureFamilyWorkspace::new(plan.pressure)?,
            attempts: 0,
            next: 0,
            failed: false,
        })
    }
    /// Remaining aggregate attempts; child failure prevents further use even when positive.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.attempts
    }
    /// Complete actual charges; malformed pre-binding calls charge only the aggregate attempt.
    pub fn charged_work(&self) -> ProbeDiagnosticWork {
        ProbeDiagnosticWork {
            attempts: self.attempts,
            physical: self.physical.charged_work(),
            pressure: self.pressure.charged_work(),
        }
    }
    /// Next exact physical observation time; no missed probe is skipped automatically.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan
            .probes
            .tested_times()
            .as_slice()
            .get(self.next)
            .copied()
    }
    /// A numerical child failure permanently invalidates this consumer.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Bind the matching complete probe, then compute all thirty quantity/pair findings.
    /// A malformed request consumes an aggregate attempt before any child is called. Any child
    /// failure terminates the consumer and publishes nothing; earlier legal commits are retained.
    pub fn measure(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<ProbePhysicalSample, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.attempts += 1;
        let origin = self.plan.probes.require_sample(family, self.next)?;
        match self.compute(family, origin) {
            Ok(result) => {
                self.next += 1;
                Ok(result)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn compute(
        &mut self,
        family: &ProbeFamily<'_>,
        origin: ProbeSample,
    ) -> Result<ProbePhysicalSample, FamilyError> {
        let [a, b, c, d] = self.physical.probe_quantities(family)?;
        let [e, f] = self.pressure.probe_quantities(family)?;
        Ok(ProbePhysicalSample {
            origin,
            samples: self.plan.sample_layout(),
            quantities: [a, b, c, d, e, f],
        })
    }
}
