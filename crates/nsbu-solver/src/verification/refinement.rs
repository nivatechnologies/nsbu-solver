//! Numerical comparison rules do not establish the identity or provenance of a trajectory.
use super::VerificationError;

/// Evidence required by a particular channel; sensitivity is not a convergence-order claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// One independently improved evaluation, such as the prescribed precision comparison.
    Sensitivity,
    /// Two successive differences from at least three independently resolved settings.
    Refinement,
}

/// Supplied empirical differences. The experiment must independently verify their provenance.
#[derive(Debug, Clone, Copy)]
pub enum Evidence {
    /// No measurement exists; missing uncertainty cannot be represented by zero.
    Missing,
    /// One measured change between two evaluations.
    Pair(f64),
    /// Successive differences from three settings, in coarse-to-fine order.
    Sequence([f64; 2]),
    /// A plateau supported by separately documented analysis, not inferred from agreement.
    Floor {
        /// Two observed changes, both controlled by the independently supported floor.
        changes: [f64; 2],
        /// Explicit nonnegative estimate, not a certified enclosure.
        bound: f64,
        /// Identifier of the supporting analysis; zero is the missing-identifier sentinel.
        analysis: [u8; 32],
    },
}

/// Channel-level finding only; none of these findings accepts a PDE window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finding {
    /// Missing measurements or insufficient refinement levels.
    MissingEvidence,
    /// The measured discrepancy is not strictly below its allocated budget.
    AboveBudget,
    /// Measurements do not show the declared decrease or a supported subordinate floor.
    ConvergenceInconclusive,
    /// An independently improved pair changes by less than the sensitivity allocation.
    SensitivityBelowBudget,
    /// Successive changes show the declared decrease and fit the allocation.
    RefinementBelowBudget,
    /// A separately supported floor controls observations and is strictly subordinate.
    SubordinateFloor,
    /// The independently measured reference discrepancy is strictly below its total tolerance.
    TrackingBelowBudget,
}

impl Finding {
    /// This finding passes its numerical check only; identity and lineage remain separate obligations.
    pub fn passes(self) -> bool {
        matches!(
            self,
            Self::SensitivityBelowBudget
                | Self::RefinementBelowBudget
                | Self::SubordinateFloor
                | Self::TrackingBelowBudget
        )
    }
}

/// Immutable numerical rule, declared before observing the experiment's results.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    budget: f64,
    reduction: f64,
    floor_budget: f64,
    requirement: Requirement,
}
impl Rule {
    /// Frozen channel allocation in the observable's declared units.
    pub fn budget(self) -> f64 {
        self.budget
    }

    /// Frozen minimum comparison requirement.
    pub fn requirement(self) -> Requirement {
        self.requirement
    }

    /// Frozen strict reduction threshold; required for complete protocol identity.
    pub fn maximum_reduction_ratio(self) -> f64 {
        self.reduction
    }

    /// Effective absolute subordinate-floor budget after constructor arithmetic.
    pub fn subordinate_floor_budget(self) -> f64 {
        self.floor_budget
    }

    /// Require a positive finite budget and fractions strictly between zero and one.
    /// `floor_fraction` defines strict subordination to the channel allocation.
    pub fn new(
        budget: f64,
        maximum_reduction_ratio: f64,
        floor_fraction: f64,
        requirement: Requirement,
    ) -> Result<Self, VerificationError> {
        positive(budget)?;
        fraction(maximum_reduction_ratio)?;
        fraction(floor_fraction)?;
        let floor_budget = budget * floor_fraction;
        positive(floor_budget)?;
        Ok(Self {
            budget,
            reduction: maximum_reduction_ratio,
            floor_budget,
            requirement,
        })
    }

    /// Review supplied finite differences without inventing missing uncertainty.
    pub fn evaluate(self, evidence: Evidence) -> Result<Finding, VerificationError> {
        match evidence {
            Evidence::Missing => Ok(Finding::MissingEvidence),
            Evidence::Pair(change) => {
                magnitude(change)?;
                if self.requirement == Requirement::Refinement {
                    Ok(Finding::MissingEvidence)
                } else if change >= self.budget {
                    Ok(Finding::AboveBudget)
                } else {
                    Ok(Finding::SensitivityBelowBudget)
                }
            }
            Evidence::Sequence(changes) => self.sequence(changes),
            Evidence::Floor {
                changes,
                bound,
                analysis,
            } => self.floor(changes, bound, analysis),
        }
    }

    fn sequence(self, [coarse, fine]: [f64; 2]) -> Result<Finding, VerificationError> {
        magnitude(coarse)?;
        magnitude(fine)?;
        if fine >= self.budget {
            Ok(Finding::AboveBudget)
        } else if fine < coarse * self.reduction {
            Ok(Finding::RefinementBelowBudget)
        } else {
            Ok(Finding::ConvergenceInconclusive)
        }
    }

    fn floor(
        self,
        changes: [f64; 2],
        bound: f64,
        analysis: [u8; 32],
    ) -> Result<Finding, VerificationError> {
        magnitude(bound)?;
        for change in changes {
            magnitude(change)?;
        }
        if analysis == [0; 32] {
            return Err(VerificationError::MissingAnalysis);
        }
        if changes.into_iter().any(|value| value >= self.budget) {
            Ok(Finding::AboveBudget)
        } else if bound >= self.floor_budget || changes.into_iter().any(|value| value > bound) {
            Ok(Finding::ConvergenceInconclusive)
        } else {
            Ok(Finding::SubordinateFloor)
        }
    }
}

pub(super) fn magnitude(value: f64) -> Result<(), VerificationError> {
    if !value.is_finite() || value < 0.0 {
        Err(VerificationError::InvalidValue)
    } else {
        Ok(())
    }
}
pub(super) fn positive(value: f64) -> Result<(), VerificationError> {
    magnitude(value)?;
    if value == 0.0 {
        Err(VerificationError::InvalidValue)
    } else {
        Ok(())
    }
}
fn fraction(value: f64) -> Result<(), VerificationError> {
    positive(value)?;
    if value >= 1.0 {
        Err(VerificationError::InvalidValue)
    } else {
        Ok(())
    }
}
