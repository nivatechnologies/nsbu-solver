//! Bounded off-stage comparisons from actual accepted histories on three temporal branches.
use super::{FamilyError, FamilyPlan, SmoothFamily};
use nsbu_solver::{
    diagnostics::comparison::{BandComparison, ComparisonPlan},
    domain::{Domain, TickClock},
    verification::{
        reconstruction::{OffStageProbe, ProbeRefinement},
        VerificationError,
    },
    Complex64, SolverError,
};
type Field = [Vec<Complex64>; 3];

/// Actual accepted-node geometry and complete-band changes at one common off-stage probe.
#[derive(Debug, Clone, Copy)]
pub struct ReconstructionComparison {
    geometry: ProbeRefinement,
    values: [BandComparison; 2],
    derivatives: [BandComparison; 2],
}
impl ReconstructionComparison {
    /// Nested accepted histories; the probe is strictly off every full/two-half stage set.
    pub fn geometry(self) -> ProbeRefinement {
        self.geometry
    }
    /// Coarse/medium and medium/fine interpolated velocity differences on the finest grid.
    pub fn values(self) -> [BandComparison; 2] {
        self.values
    }
    /// Physical-time derivative differences, retaining the same complete comparison band.
    pub fn derivatives(self) -> [BandComparison; 2] {
        self.derivatives
    }
}
/// Independent interpolation scratch. The family and its accepted histories stay borrowed.
pub struct ReconstructionWorkspace {
    domain: Domain,
    maximum: usize,
    attempts: usize,
    value: [Field; 2],
    derivative: [Field; 2],
}
impl ReconstructionWorkspace {
    /// Complete fixed storage and bounded coefficient work, before allocating any scratch.
    /// Work counts weighted field visits and comparison visits, not primitive FLOPs.
    pub fn reservation(
        domain: Domain,
        maximum_probes: usize,
    ) -> Result<(usize, usize), SolverError> {
        if maximum_probes == 0 {
            return Err(SolverError::ResourceLimit);
        }
        let n = domain.layout().half_len();
        let bytes = n
            .checked_mul(12 * std::mem::size_of::<Complex64>())
            .and_then(|v| v.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)?;
        let work = n
            .checked_mul(100)
            .and_then(|v| v.checked_add(128))
            .and_then(|v| v.checked_mul(maximum_probes))
            .ok_or(SolverError::SizeOverflow)?;
        Ok((bytes, work))
    }
    /// Reserve two vector interpolants and their derivatives on the family's finest grid.
    pub fn new(
        plan: FamilyPlan<'_>,
        maximum_probes: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let domain = plan.branches[2].plan.resources().domain();
        if Self::reservation(domain, maximum_probes)?.0 > cap {
            return Err(SolverError::ResourceLimit);
        }
        let n = domain.layout().half_len();
        Ok(Self {
            domain,
            maximum: maximum_probes,
            attempts: 0,
            value: [field(n)?, field(n)?],
            derivative: [field(n)?, field(n)?],
        })
    }
    /// Remaining attempts; malformed probes consume one slot without changing physical history.
    pub fn remaining(&self) -> usize {
        self.maximum - self.attempts
    }
    /// Compare the independently evolved H0/H1/H2 histories at one exact off-stage time.
    /// The existing rings must all cover this probe; no extrapolation or state reset is allowed.
    pub fn measure(
        &mut self,
        family: &SmoothFamily<'_>,
        probe: TickClock,
    ) -> Result<ReconstructionComparison, FamilyError> {
        if self.attempts == self.maximum {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.attempts += 1;
        if family.plan.branches[2].plan.resources().domain() != self.domain {
            return Err(SolverError::InvalidDomain.into());
        }
        let geometry = geometry(family, probe)?;
        let first = self.compare(family, [3, 4], probe)?;
        let second = self.compare(family, [4, 2], probe)?;
        Ok(ReconstructionComparison {
            geometry,
            values: [first.0, second.0],
            derivatives: [first.1, second.1],
        })
    }
    fn compare(
        &mut self,
        family: &SmoothFamily<'_>,
        branches: [usize; 2],
        probe: TickClock,
    ) -> Result<(BandComparison, BandComparison), FamilyError> {
        for (slot, branch) in branches.into_iter().enumerate() {
            for axis in 0..3 {
                family.branches[branch].observer().reconstruct(
                    probe,
                    axis,
                    &mut self.value[slot][axis],
                    &mut self.derivative[slot][axis],
                )?;
            }
        }
        let comparison = ComparisonPlan::new(self.domain, self.domain)?;
        let values = comparison.compare(
            self.value[0].each_ref().map(Vec::as_slice),
            self.value[1].each_ref().map(Vec::as_slice),
        )?;
        let derivatives = comparison.compare(
            self.derivative[0].each_ref().map(Vec::as_slice),
            self.derivative[1].each_ref().map(Vec::as_slice),
        )?;
        Ok((values, derivatives))
    }
}
fn geometry(family: &SmoothFamily<'_>, probe: TickClock) -> Result<ProbeRefinement, FamilyError> {
    let node = |branch: usize| {
        let nodes = family.branches[branch]
            .observer()
            .last_accepted_clocks()
            .ok_or(VerificationError::MissingReconstruction)?;
        OffStageProbe::new(nodes, probe)
    };
    Ok(ProbeRefinement::new([node(3)?, node(4)?, node(2)?])?)
}
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut data = Vec::new();
    data.try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    data.resize(n, Complex64::new(0.0, 0.0));
    Ok(data)
}
