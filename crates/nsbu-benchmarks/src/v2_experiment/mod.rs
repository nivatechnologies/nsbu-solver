//! Diagnostic exact-v2 refinement family, with six independent from-rest runs.
//!
//! This module reports measured full-band differences only. Force, arithmetic,
//! sampling, reference and other missing channels remain open, so no PDE
//! acceptance or qualification claim follows from these samples.
pub mod binding;
pub mod coverage;
pub mod diagnostic;
mod identity;
pub mod physical;
mod plan;
pub mod pressure;
pub mod pressure_reference;
pub mod probes;
pub mod sampling;

pub mod reference;
pub mod review_adapter;
pub mod review_profile;
use crate::v2_run::Run;
use nsbu_solver::{
    diagnostics::comparison::{BandComparison, ComparisonPlan},
    domain::TickClock,
    experiment::control::Outcome,
    SolverError,
};
pub use plan::{FamilyBounds, FamilyPlan, FamilySettings};

const PAIRS: [(usize, usize); 5] = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];

/// Terminal family errors; committed branch state is retained on failure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FamilyError {
    /// Family settings or synchronized clocks are invalid.
    InvalidFamily,
    /// A solver operation refused or exhausted its bounded resources.
    Numerical(SolverError),
    /// A checked verification input was invalid.
    Verification(nsbu_solver::verification::VerificationError),
    /// A branch stopped with a recorded outcome; the family is terminal.
    BranchStopped {
        /// Zero-based stopped branch slot.
        branch: usize,
        /// Recorded terminal outcome.
        outcome: Outcome,
    },
    /// A previous advance failed and cannot be retried.
    Terminated,
}
impl From<SolverError> for FamilyError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}
impl From<nsbu_solver::verification::VerificationError> for FamilyError {
    fn from(error: nsbu_solver::verification::VerificationError) -> Self {
        Self::Verification(error)
    }
}

/// One synchronized set of full-band diagnostic comparisons.
#[derive(Debug, Clone, Copy)]
pub struct RefinementSample {
    clock: TickClock,
    comparisons: [BandComparison; 5],
    identity: [u8; 32],
}
impl RefinementSample {
    /// Synchronized exact clock of the comparison.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Complete fine-band spatial comparisons.
    pub fn space(self) -> [BandComparison; 2] {
        [self.comparisons[0], self.comparisons[1]]
    }
    /// Complete fine-band temporal comparisons.
    pub fn time(self) -> [BandComparison; 2] {
        [self.comparisons[2], self.comparisons[3]]
    }
    /// Complete fine-band CM/HO comparison.
    pub fn method(self) -> BandComparison {
        self.comparisons[4]
    }
    /// Family identity carried by this sample.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
}

/// Six independently owned exact-v2 runs sharing only immutable settings.
pub struct V2Family<'a> {
    plan: FamilyPlan<'a>,
    branches: [Run; 6],
    next: usize,
    failed: bool,
}
impl<'a> V2Family<'a> {
    /// Allocate all six admitted branches from exact rest.
    pub fn new(plan: FamilyPlan<'a>) -> Result<Self, FamilyError> {
        let build = |index| Run::from_rest(plan.branches[index]);
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
    /// Read-only access to one independently owned branch.
    pub fn branch(&self, index: usize) -> Option<&Run> {
        self.branches.get(index)
    }
    /// Immutable admission plan.
    pub fn plan(&self) -> FamilyPlan<'a> {
        self.plan
    }
    /// Advance all branches to the next tested time and compare actual spectra.
    pub fn advance(&mut self) -> Result<Option<RefinementSample>, FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        let Some(clock) = self.plan.times.as_slice().get(self.next).copied() else {
            return Ok(None);
        };
        let result = self.advance_to(clock);
        match result {
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
        let comparisons = PAIRS
            .map(|pair| self.compare(pair))
            .map(|result| result.map_err(FamilyError::from));
        Ok(RefinementSample {
            clock,
            comparisons: [
                comparisons[0]?,
                comparisons[1]?,
                comparisons[2]?,
                comparisons[3]?,
                comparisons[4]?,
            ],
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
