//! Complete per-observable numerical findings; no mask or channel can hide another failure.
use super::{
    budget::{Budget, CHANNELS},
    refinement::{magnitude, Evidence, Finding},
    VerificationError,
};
use crate::domain::TickClock;

/// Primary representation declaration; provenance validation must confirm this declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonScope {
    /// Complete fine-band field or corresponding physical observable, without phase/location alignment.
    FullField,
    /// Diagnostic comparison restricted to common coarse/fine modes; insufficient for primary review.
    CommonBand,
    /// A transformed/aligned comparison, which cannot replace the primary unaligned result.
    Aligned,
}

/// Supplied evidence for one observable at one exact time.
/// Independent full-band/current-grid identity and actual-state provenance are validated separately.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'a> {
    /// The next exact time required by the review schedule.
    pub time: TickClock,
    /// The next required observable key.
    pub key: u32,
    /// Only full-field, unaligned comparisons may enter the primary measurement ledger.
    pub comparison: ComparisonScope,
    /// Measured reference discrepancy; None is missing evidence, not zero error.
    pub tracking_error: Option<f64>,
    /// Every channel, in the published CHANNELS order.
    pub channels: &'a [Evidence; 11],
}

/// All numerical findings for one observation, including simultaneous failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationFindings {
    /// Reference discrepancy check against the frozen total tolerance.
    pub tracking: Finding,
    /// Individual channel results in the published CHANNELS order.
    pub channels: [Finding; 11],
}
impl ObservationFindings {
    /// All numerical checks pass; this is not a trajectory/window qualification.
    pub fn passes(self) -> bool {
        self.tracking.passes() && self.channels.into_iter().all(Finding::passes)
    }
}

pub(super) fn evaluate(
    budget: Budget,
    observation: Observation<'_>,
) -> Result<ObservationFindings, VerificationError> {
    let tracking = match observation.tracking_error {
        None => Finding::MissingEvidence,
        Some(value) => {
            magnitude(value)?;
            if value >= budget.total() {
                Finding::AboveBudget
            } else {
                Finding::TrackingBelowBudget
            }
        }
    };
    let mut channels = [Finding::MissingEvidence; 11];
    for (channel, finding) in CHANNELS.into_iter().zip(&mut channels) {
        *finding = budget
            .rule(channel)
            .evaluate(observation.channels[channel as usize])?;
    }
    Ok(ObservationFindings { tracking, channels })
}
