//! Bounded exact-v2 force-sampling refinement with three private from-rest runs.
//!
//! Retained velocity resolution, integrator, step and endpoint are identical in every
//! branch. Only the nested prescribed-force sampling grid changes. Reported values are
//! measured full-band trajectory differences; they do not establish force sufficiency.
mod identity;
mod plan;
use crate::v2_run::Run;
use nsbu_solver::{
    diagnostics::comparison::{BandComparison, ComparisonPlan},
    domain::TickClock,
    experiment::control::Outcome,
    SolverError,
};
pub use plan::{ForceFamilyBounds, ForceFamilyPlan, ForceFamilySettings};

const PAIRS: [(usize, usize); 2] = [(0, 1), (1, 2)];

/// Terminal errors; a failed child retains its committed state and charged work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForceFamilyError {
    /// Family settings or synchronized clocks are invalid.
    InvalidFamily,
    /// A solver operation refused or exhausted its bounded resources.
    Numerical(SolverError),
    /// A checked tested-time input was invalid.
    Verification(nsbu_solver::verification::VerificationError),
    /// A branch stopped with its actual recorded outcome.
    BranchStopped {
        /// Zero-based force-grid branch.
        branch: usize,
        /// Terminal outcome recorded by the child run.
        outcome: Outcome,
    },
    /// A previous family advance failed, so no fresh allowance is available.
    Terminated,
}
impl From<SolverError> for ForceFamilyError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}
impl From<nsbu_solver::verification::VerificationError> for ForceFamilyError {
    fn from(error: nsbu_solver::verification::VerificationError) -> Self {
        Self::Verification(error)
    }
}

/// Synchronized M0/M1 and M1/M2 full-band spectral trajectory differences.
#[derive(Debug, Clone, Copy)]
pub struct ForceRefinementSample {
    clock: TickClock,
    comparisons: [BandComparison; 2],
    identity: [u8; 32],
}
impl ForceRefinementSample {
    /// Exact accepted clock shared by all three trajectories.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Pair order is M0/M1 followed by M1/M2. `full.l2` measures velocity and
    /// `full.h1` includes every first spectral derivative on the fixed retained band.
    pub fn comparisons(self) -> [BandComparison; 2] {
        self.comparisons
    }
    /// Immutable family identity carried by this sample.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
}

/// Three independently allocated exact-v2 runs with force sampling as the sole varying input.
pub struct ForceFamily<'a> {
    plan: ForceFamilyPlan<'a>,
    branches: [Run; 3],
    next: usize,
    failed: bool,
}
impl<'a> ForceFamily<'a> {
    /// Allocate every admitted child from exact zero.
    pub fn from_rest(plan: ForceFamilyPlan<'a>) -> Result<Self, ForceFamilyError> {
        let build = |index| Run::from_rest(plan.branches[index]);
        Ok(Self {
            plan,
            branches: [build(0)?, build(1)?, build(2)?],
            next: 0,
            failed: false,
        })
    }
    /// Read-only child access; callers cannot reset or replace its state.
    pub fn branch(&self, index: usize) -> Option<&Run> {
        self.branches.get(index)
    }
    /// Immutable aggregate admission.
    pub fn plan(&self) -> ForceFamilyPlan<'a> {
        self.plan
    }
    /// Advance all children to the next manifest clock, then compare actual spectra.
    pub fn advance(&mut self) -> Result<Option<ForceRefinementSample>, ForceFamilyError> {
        if self.failed {
            return Err(ForceFamilyError::Terminated);
        }
        let Some(clock) = self.plan.times.as_slice().get(self.next).copied() else {
            return Ok(None);
        };
        match self.advance_to(clock) {
            Ok(sample) => {
                self.next += 1;
                Ok(Some(sample))
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn advance_to(&mut self, clock: TickClock) -> Result<ForceRefinementSample, ForceFamilyError> {
        for (index, branch) in self.branches.iter_mut().enumerate() {
            while branch.state().clock().elapsed() < clock.elapsed() {
                let outcome = branch.step()?;
                if !matches!(outcome, Outcome::Committed(_)) {
                    return Err(ForceFamilyError::BranchStopped {
                        branch: index,
                        outcome,
                    });
                }
            }
            if branch.state().clock() != clock {
                return Err(ForceFamilyError::InvalidFamily);
            }
        }
        let comparisons = PAIRS.map(|pair| self.compare(pair));
        Ok(ForceRefinementSample {
            clock,
            comparisons: [comparisons[0]?, comparisons[1]?],
            identity: self.plan.identity(),
        })
    }
    fn compare(&self, (a, b): (usize, usize)) -> Result<BandComparison, SolverError> {
        let left = self.branches[a].state();
        let right = self.branches[b].state();
        ComparisonPlan::new(left.plan().domain(), right.plan().domain())?.compare(
            [left.component(0)?, left.component(1)?, left.component(2)?],
            [
                right.component(0)?,
                right.component(1)?,
                right.component(2)?,
            ],
        )
    }
}
