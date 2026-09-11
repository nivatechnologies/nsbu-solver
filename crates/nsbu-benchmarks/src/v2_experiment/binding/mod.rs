//! Read-only accepted-node provenance binding across independent exact-v2 owners.
mod plan;
mod report;
use super::{probes::ProbeFamily, FamilyError, V2Family};
use crate::v2_run::Origin;
use nsbu_solver::{domain::SpectralState, SolverError};
pub use plan::{NodeBindingBounds, NodeBindingPlan, NodeBindingWork};
pub use report::{AcceptedNodeProvenance, NodeBindingSample, NodeBindingStatus};

/// Binding failures preserve family, resource and terminal origins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeBindingError {
    /// Ordinary/probe identity, clock, branch or provenance mismatch.
    Family(FamilyError),
    /// Bounded work exhaustion or malformed spectra.
    Numerical(SolverError),
    /// A located accepted node contradicted its immutable branch, clock or state provenance.
    InvalidAcceptedNode {
        /// Fixed family branch slot containing malformed provenance.
        branch: usize,
    },
    /// A previous in-progress binding computation failed.
    Terminated,
}
impl From<FamilyError> for NodeBindingError {
    fn from(error: FamilyError) -> Self {
        Self::Family(error)
    }
}
impl From<SolverError> for NodeBindingError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

/// Fixed-work coordinator that owns no numerical state or reconstruction scratch.
pub struct NodeBindingWorkspace<'a> {
    plan: NodeBindingPlan<'a>,
    charged: NodeBindingWork,
    next: usize,
    failed: bool,
}
impl<'a> NodeBindingWorkspace<'a> {
    /// Construct the allocation-free coordinator after joint admission.
    pub fn new(plan: NodeBindingPlan<'a>) -> Self {
        Self {
            plan,
            charged: NodeBindingWork::default(),
            next: 0,
            failed: false,
        }
    }
    /// Next ordinary accepted clock required by the binding manifest.
    pub fn next_time(&self) -> Option<nsbu_solver::domain::TickClock> {
        self.plan.family.times().as_slice().get(self.next).copied()
    }
    /// Remaining admitted requests, including refusals.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Worst-case work charged for complete and refused requests.
    pub fn charged_work(&self) -> NodeBindingWork {
        self.charged
    }
    /// Bind six ordinary accepted states only to exact nodes still retained by the probe family.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
        probes: &ProbeFamily<'_>,
    ) -> Result<NodeBindingSample, NodeBindingError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        self.validate_probe(probes)?;
        match self.compute(family, probes, clock) {
            Ok(sample) => {
                self.next += 1;
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), NodeBindingError> {
        if self.failed {
            return Err(NodeBindingError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.node_lookups += self.plan.per_attempt.node_lookups;
        self.charged.coefficient_visits += self.plan.per_attempt.coefficient_visits;
        Ok(())
    }
    fn validate_probe(&self, probes: &ProbeFamily<'_>) -> Result<(), NodeBindingError> {
        if probes.is_terminated() {
            return Err(FamilyError::Terminated.into());
        }
        if probes.plan().identity() != self.plan.probes.identity()
            || probes.plan().family_plan().identity() != self.plan.family.identity()
        {
            return Err(FamilyError::InvalidFamily.into());
        }
        Ok(())
    }
    fn compute(
        &self,
        family: &V2Family<'_>,
        probes: &ProbeFamily<'_>,
        clock: nsbu_solver::domain::TickClock,
    ) -> Result<NodeBindingSample, NodeBindingError> {
        let mut branches = [NodeBindingStatus::MissingRetainedNode; 6];
        for (branch, finding) in branches.iter_mut().enumerate() {
            *finding = self.branch(family, probes, branch, clock)?;
        }
        Ok(NodeBindingSample {
            clock,
            family_identity: self.plan.family.identity(),
            probe_identity: self.plan.probes.identity(),
            branches,
        })
    }
    fn branch(
        &self,
        family: &V2Family<'_>,
        probes: &ProbeFamily<'_>,
        branch: usize,
        clock: nsbu_solver::domain::TickClock,
    ) -> Result<NodeBindingStatus, NodeBindingError> {
        let ordinary = family
            .branch(branch)
            .ok_or(SolverError::InvalidPayload)?
            .state();
        let Some(probe) = probes.accepted_node(branch, clock) else {
            return Ok(NodeBindingStatus::MissingRetainedNode);
        };
        validate(&probe, ordinary, branch, clock, self.plan)?;
        Ok(NodeBindingStatus::Compared(AcceptedNodeProvenance {
            clock: probe.node.clock,
            epoch: probe.node.epoch,
            accepted_steps: probe.node.steps,
            origin: probe.origin,
            coefficients_equal: equal(probe.node.value, ordinary)?,
        }))
    }
}

fn validate(
    probe: &super::probes::ProbeAcceptedNode<'_>,
    ordinary: &SpectralState,
    branch: usize,
    clock: nsbu_solver::domain::TickClock,
    plan: NodeBindingPlan<'_>,
) -> Result<(), NodeBindingError> {
    let observed = NodeIdentity {
        branch: probe.branch,
        family: probe.family_identity,
        probes: probe.probe_identity,
        origin: probe.origin,
        domain: probe.node.domain,
        clock: probe.node.clock,
        epoch: probe.node.epoch,
        steps: probe.node.steps,
    };
    let expected = NodeIdentity {
        branch,
        family: plan.family.identity(),
        probes: plan.probes.identity(),
        origin: Origin::InternalFromRest,
        domain: ordinary.plan().domain(),
        clock,
        epoch: ordinary.epoch(),
        steps: ordinary.accepted_steps(),
    };
    if observed != expected {
        return Err(NodeBindingError::InvalidAcceptedNode { branch });
    }
    Ok(())
}

#[derive(PartialEq)]
struct NodeIdentity {
    branch: usize,
    family: [u8; 32],
    probes: [u8; 32],
    origin: Origin,
    domain: nsbu_solver::domain::Domain,
    clock: nsbu_solver::domain::TickClock,
    epoch: nsbu_solver::domain::Epoch,
    steps: u128,
}

fn equal(
    values: [&[nsbu_solver::Complex64]; 3],
    state: &SpectralState,
) -> Result<bool, SolverError> {
    for (axis, values) in values.into_iter().enumerate() {
        let ordinary = state.component(axis)?;
        if !equal_words(ordinary, values) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn equal_words(left: &[nsbu_solver::Complex64], right: &[nsbu_solver::Complex64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}

#[cfg(test)]
mod tests {
    use super::equal_words;
    use nsbu_solver::Complex64;

    #[test]
    fn bitwise_control_preserves_false_for_finite_and_signed_zero_differences() {
        let original = [Complex64::new(1.0, 0.0), Complex64::new(-2.0, 3.0)];
        assert!(equal_words(&original, &original));
        let mut finite = original;
        finite[1].im = f64::from_bits(finite[1].im.to_bits() + 1);
        assert!(!equal_words(&original, &finite));
        let signed = [Complex64::new(1.0, -0.0), original[1]];
        assert!(!equal_words(&original, &signed));
        assert!(!equal_words(&original, &original[..1]));
    }
}
