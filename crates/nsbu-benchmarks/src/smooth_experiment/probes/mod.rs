//! Stream early and late off-stage probes before bounded accepted histories are overwritten.
pub mod diagnostics;
mod plan;
mod report;
#[cfg(test)]
mod tests;
use super::{FamilyError, SmoothFamily, PAIRS};
use crate::smooth_run::ReconstructedRun;
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan, domain::TickClock, experiment::control::Outcome,
    Complex64, SolverError,
};
pub use plan::{ProbeBounds, ProbePlan, ProbeWork};
pub use report::{ProbeFields, ProbeOrigin, ProbeSample};
type Field = [Vec<Complex64>; 3];
struct Interpolant {
    value: Field,
    derivative: Field,
}

/// Six independently evolved owners and separately admitted full-band interpolation scratch.
/// All evolution is from rest; analytical reference fields are absent from this interface.
pub struct ProbeFamily<'a> {
    plan: ProbePlan<'a>,
    branches: [ReconstructedRun; 6],
    scratch: [Interpolant; 6],
    next: usize,
    charged: ProbeWork,
    failed: bool,
    current: Option<ProbeSample>,
}
impl<'a> ProbeFamily<'a> {
    /// Allocate only after complete joint admission of owners, scratch and report capacity.
    pub fn new(plan: ProbePlan<'a>) -> Result<Self, FamilyError> {
        let SmoothFamily { branches, .. } = SmoothFamily::new(plan.family)?;
        let [a, b, c, d, e, f] = plan.family.branches.map(|branch| {
            let n = branch.plan.resources().domain().layout().half_len();
            Ok::<_, SolverError>(Interpolant {
                value: field(n)?,
                derivative: field(n)?,
            })
        });
        Ok(Self {
            plan,
            branches,
            scratch: [a?, b?, c?, d?, e?, f?],
            next: 0,
            charged: ProbeWork::default(),
            failed: false,
            current: None,
        })
    }
    /// Unchanged immutable admission and exact probe schedule.
    pub fn plan(&self) -> ProbePlan<'a> {
        self.plan
    }
    /// Actual independently integrated owner; its clock may exceed the current probe time.
    pub fn branch(&self, index: usize) -> Option<&ReconstructedRun> {
        self.branches.get(index)
    }
    /// Charged complete probe attempts; original integration/provider work remains with each owner.
    pub fn charged_work(&self) -> ProbeWork {
        self.charged
    }
    /// Next physical observation time; no earlier sample can be silently skipped.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.times.as_slice().get(self.next).copied()
    }
    /// Failure permanently terminates this family without discarding earlier legal commits.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Only complete current scratch is exposed. A failed next probe invalidates these views.
    pub fn fields(&self, index: usize) -> Option<ProbeFields<'_>> {
        let sample = self.current?;
        let branch = self.branches.get(index)?;
        let scratch = &self.scratch[index];
        Some(ProbeFields {
            domain: branch.state().plan().domain(),
            clock: sample.clock,
            origin: sample.origins[index],
            value: scratch.value.each_ref().map(Vec::as_slice),
            derivative: scratch.derivative.each_ref().map(Vec::as_slice),
        })
    }
    /// Obtain the next probe using finite accepted-history lookahead and then compare every branch.
    /// Initial probes require two accepted macro steps per owner. No interpolated value is assigned
    /// to an integrated state. A refusal retains all prior legal commits and publishes no partial view.
    pub fn advance(&mut self) -> Result<Option<ProbeSample>, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        let Some(clock) = self.next_time() else {
            return Ok(None);
        };
        self.current = None;
        if self.charged.attempts == self.plan.bounds.work.attempts {
            self.failed = true;
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.weighted_visits += self.plan.visits_per_attempt;
        match self.sample(clock) {
            Ok(sample) => {
                self.next += 1;
                self.current = Some(sample);
                Ok(Some(sample))
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn sample(&mut self, clock: TickClock) -> Result<ProbeSample, FamilyError> {
        let mut origins = [ProbeOrigin {
            accepted_nodes: [clock; 3],
            state_clock: clock,
        }; 6];
        for (index, origin) in origins.iter_mut().enumerate() {
            *origin = self.interpolate(index, clock)?;
        }
        let values = self.compare(false)?;
        let derivatives = self.compare(true)?;
        Ok(ProbeSample {
            clock,
            origins,
            values,
            derivatives,
        })
    }
    fn interpolate(&mut self, index: usize, clock: TickClock) -> Result<ProbeOrigin, FamilyError> {
        let branch = &mut self.branches[index];
        let step = self.plan.family.branches[index]
            .plan
            .configuration()
            .limits
            .step_ticks;
        let nodes = plan::nodes(clock, step)?;
        while branch.state().clock().elapsed() < nodes[2].elapsed() {
            let outcome = branch.step()?;
            if !matches!(outcome, Outcome::Committed(_)) {
                return Err(FamilyError::BranchStopped {
                    branch: index,
                    outcome,
                });
            }
        }
        if branch.observer().last_accepted_clocks() != Some(nodes)
            || branch.state().clock() != nodes[2]
        {
            return Err(FamilyError::InvalidFamily);
        }
        let scratch = &mut self.scratch[index];
        for axis in 0..3 {
            branch.observer().reconstruct(
                clock,
                axis,
                &mut scratch.value[axis],
                &mut scratch.derivative[axis],
            )?;
        }
        Ok(ProbeOrigin {
            accepted_nodes: nodes,
            state_clock: branch.state().clock(),
        })
    }
    fn compare(
        &self,
        derivative: bool,
    ) -> Result<[nsbu_solver::diagnostics::comparison::BandComparison; 5], SolverError> {
        let compare = |(a, b): (usize, usize)| {
            let select = |index: usize| {
                if derivative {
                    &self.scratch[index].derivative
                } else {
                    &self.scratch[index].value
                }
            };
            ComparisonPlan::new(
                self.branches[a].state().plan().domain(),
                self.branches[b].state().plan().domain(),
            )?
            .compare(
                select(a).each_ref().map(Vec::as_slice),
                select(b).each_ref().map(Vec::as_slice),
            )
        };
        let [a, b, c, d, e] = PAIRS.map(compare);
        Ok([a?, b?, c?, d?, e?])
    }
}
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    result.resize(n, Complex64::new(0.0, 0.0));
    Ok(result)
}
