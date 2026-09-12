//! Stream exact-v2 off-stage probes before bounded accepted histories are overwritten.
pub mod balances;
pub mod physical;
mod plan;
mod report;
pub mod residuals;
use super::{FamilyError, PAIRS};
use crate::v2_run::ReconstructedRun;
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan, domain::TickClock, experiment::control::Outcome,
    Complex64, SolverError,
};
pub use plan::{ProbeBounds, ProbePlan, ProbeWork};
pub use report::{ProbeFields, ProbeOrigin, ProbeSample};
use sha2::{Digest, Sha256};

pub(super) struct ProbeAcceptedNode<'a> {
    pub(super) branch: usize,
    pub(super) family_identity: [u8; 32],
    pub(super) probe_identity: [u8; 32],
    pub(super) origin: crate::v2_run::Origin,
    pub(super) node: crate::smooth_observer::reconstruction::AcceptedNodeView<'a>,
}

type Field = [Vec<Complex64>; 3];
struct Interpolant {
    value: Field,
    derivative: Field,
}

/// Six independently evolved exact-v2 owners and full-band interpolation scratch.
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
    /// Allocate only after complete joint admission. Every branch begins at exact rest.
    pub fn new(plan: ProbePlan<'a>) -> Result<Self, FamilyError> {
        let [a, b, c, d, e, f] = plan.branches.map(ReconstructedRun::from_rest);
        let branches = [a?, b?, c?, d?, e?, f?];
        let [a, b, c, d, e, f] = plan.branches.map(|branch| {
            let n = branch.resources().domain().layout().half_len();
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
    /// Immutable admission and probe manifest.
    pub fn plan(&self) -> ProbePlan<'a> {
        self.plan
    }
    /// One independently integrated reconstruction-capable branch.
    pub fn branch(&self, index: usize) -> Option<&ReconstructedRun> {
        self.branches.get(index)
    }
    /// Charged complete attempts; branch integration work remains with each owner.
    pub fn charged_work(&self) -> ProbeWork {
        self.charged
    }
    /// Next required probe clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.times.as_slice().get(self.next).copied()
    }
    /// Whether a prior failed attempt permanently terminated this family.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Current complete reconstructed field, absent before or during failed publication.
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
    pub(super) fn accepted_node(
        &self,
        branch: usize,
        clock: TickClock,
    ) -> Option<ProbeAcceptedNode<'_>> {
        let run = self.branches.get(branch)?;
        Some(ProbeAcceptedNode {
            branch,
            family_identity: self.plan.family.identity(),
            probe_identity: self.plan.identity,
            origin: run.origin(),
            node: run.observer().accepted_node(clock)?,
        })
    }
    /// Stream the next complete probe; failures expose no partial reconstructed fields.
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
        Ok(ProbeSample {
            clock,
            origins,
            values: self.compare(false)?,
            derivatives: self.compare(true)?,
            identity: self.plan.identity,
        })
    }
    fn interpolate(&mut self, index: usize, clock: TickClock) -> Result<ProbeOrigin, FamilyError> {
        let branch = &mut self.branches[index];
        let step = branch.plan().settings().configuration.limits.step_ticks;
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

pub(super) fn identity(
    base: [u8; 32],
    times: nsbu_solver::verification::times::TestedTimes<'_>,
) -> Result<[u8; 32], SolverError> {
    let mut hash = Sha256::new();
    hash.update(b"NSBUV2PROBES0001");
    hash.update(base);
    hash.update((times.as_slice().len() as u128).to_le_bytes());
    for clock in times.as_slice() {
        hash.update(clock.exponent().to_le_bytes());
        hash.update(clock.target().to_le_bytes());
        hash.update(clock.elapsed().to_le_bytes());
        hash.update(clock.remaining().to_le_bytes());
    }
    Ok(hash.finalize().into())
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
