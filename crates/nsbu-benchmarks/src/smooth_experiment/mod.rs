//! Actual same-problem spatial, temporal and method comparisons from independent rest states.
//!
//! The private branches never import data or receive a reference-assignment interface. Reports
//! cover only the measured comparison channels; missing force, arithmetic, reference, sampling
//! and reconstruction studies still prevent a complete experiment qualification.
pub mod physical;
mod plan;
pub mod pressure;
pub mod reconstruction;
pub mod residual;
use crate::smooth_run::ReconstructedRun;
use nsbu_solver::{
    diagnostics::comparison::{BandComparison, ComparisonPlan},
    domain::TickClock,
    experiment::control::Outcome,
    SolverError,
};
pub use plan::{FamilyBounds, FamilyPlan, FamilySettings};
const PAIRS: [(usize, usize); 5] = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];

/// A refusal retains every branch's already committed history and terminates this family run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FamilyError {
    /// Grids, steps or sample times do not form the required synchronized family.
    InvalidFamily,
    /// Resource, clock or numerical operation failed.
    Numerical(SolverError),
    /// Supplied off-stage geometry or reconstruction refinement is invalid.
    Verification(nsbu_solver::verification::VerificationError),
    /// One branch recorded a local rejection or refusal; this family does not retry implicitly.
    BranchStopped {
        /// Zero-based branch slot in the documented six-branch schedule.
        branch: usize,
        /// Recorded terminal outcome, including indicators when available.
        outcome: Outcome,
    },
    /// A prior failed family advance cannot be retried with fresh allowances.
    Terminated,
}
impl From<nsbu_solver::verification::VerificationError> for FamilyError {
    fn from(error: nsbu_solver::verification::VerificationError) -> Self {
        Self::Verification(error)
    }
}
impl From<SolverError> for FamilyError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

/// Internally computed full-band differences at one synchronized exact time.
/// Only the family runner can construct this record; copying it does not alter a trajectory.
#[derive(Debug, Clone, Copy)]
pub struct RefinementSample {
    clock: TickClock,
    comparisons: [BandComparison; 5],
}
impl RefinementSample {
    /// Actual common physical clock reached by all six independent branches.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// N0/N1 and N1/N2 differences, each on the complete finer band.
    pub fn space(self) -> [BandComparison; 2] {
        [self.comparisons[0], self.comparisons[1]]
    }
    /// H0/H1 and H1/H2 differences on the finest retained grid.
    pub fn time(self) -> [BandComparison; 2] {
        [self.comparisons[2], self.comparisons[3]]
    }
    /// CM/HO difference at the finest grid and smallest macro step.
    pub fn method(self) -> BandComparison {
        self.comparisons[4]
    }
}
/// Six owned from-rest trajectories and one immutable tested-time manifest.
///
/// Slots 0..2 are CM spatial branches at the smallest step; slots 3/4 are finest-grid
/// CM branches at the two larger steps; slot 5 is the finest-grid/smallest-step HO branch.
/// Every branch owns its numerical and reconstruction scratch and spent work history.
pub struct SmoothFamily<'a> {
    plan: FamilyPlan<'a>,
    branches: [ReconstructedRun; 6],
    next: usize,
    failed: bool,
}
impl<'a> SmoothFamily<'a> {
    /// Allocate only after complete aggregate plan admission. All branches begin at exact rest.
    pub fn new(plan: FamilyPlan<'a>) -> Result<Self, FamilyError> {
        let build = |index: usize| {
            let profile = plan.branches[index].plan;
            ReconstructedRun::from_rest(
                profile.resources().domain(),
                plan.times.as_slice()[0],
                profile.configuration(),
                profile.observer_samples(),
                plan.settings.advective_limit,
                profile.resources().total(),
            )
        };
        Ok(Self {
            plan,
            branches: [
                build(0)?,
                build(1)?,
                build(2)?,
                build(3)?,
                build(4)?,
                build(5)?,
            ],
            next: 0,
            failed: false,
        })
    }
    /// Read-only branch access preserves separate origins, accepted fields and all work counters.
    pub fn branch(&self, index: usize) -> Option<&ReconstructedRun> {
        self.branches.get(index)
    }
    /// Frozen admission and work bounds for reporting.
    pub fn plan(&self) -> FamilyPlan<'a> {
        self.plan
    }
    /// Advance independently to the next declared time and compare actual complete fields.
    /// Returns None only after the time manifest is exhausted; failures are terminal.
    pub fn advance(&mut self) -> Result<Option<RefinementSample>, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
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
    fn advance_to(&mut self, clock: TickClock) -> Result<RefinementSample, FamilyError> {
        for (index, branch) in self.branches.iter_mut().enumerate() {
            while branch.state().clock().elapsed() < clock.elapsed() {
                let outcome = branch.step()?;
                if !matches!(outcome, Outcome::Committed(_)) {
                    return Err(FamilyError::BranchStopped {
                        branch: index,
                        outcome,
                    });
                }
            }
            if branch.state().clock() != clock {
                return Err(FamilyError::InvalidFamily);
            }
        }
        Ok(RefinementSample {
            clock,
            comparisons: [
                self.compare(PAIRS[0])?,
                self.compare(PAIRS[1])?,
                self.compare(PAIRS[2])?,
                self.compare(PAIRS[3])?,
                self.compare(PAIRS[4])?,
            ],
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
