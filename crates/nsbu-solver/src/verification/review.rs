//! Bounded streaming review; numerical checks cannot replace provenance or current-grid identity.
use super::{
    observation::{self, ComparisonScope, Observation, ObservationFindings},
    policy::Policies,
    reconstruction::ReconstructionSamples,
    times::TestedTimes,
    VerificationError,
};
use crate::domain::TickClock;

/// A measurement review never emits WindowAcceptedEmpirically or an enclosure claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    /// Required time/observable records are still missing.
    Incomplete,
    /// The finite attempt allowance ran out before all required records arrived.
    BudgetExceeded,
    /// All required records arrived, but one or more numerical findings failed.
    Rejected,
    /// Every numerical finding passed; same-problem and accepted-state lineage still require validation.
    ReadyForLineageReview,
}

/// Counts preserve failures even during an incomplete or exhausted review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewProgress {
    /// Number of successfully processed records, including records with failed numerical findings.
    pub reviewed: usize,
    /// Required records across the exact fine time set and complete observable inventory.
    pub required: usize,
    /// Records with one or more failed numerical findings.
    pub failed: usize,
    /// Remaining attempts; malformed records consume an attempt without advancing the schedule.
    pub attempts_left: usize,
}

/// Constant-storage streaming ledger, borrowing the complete frozen policy and time manifests.
#[derive(Debug)]
pub struct MeasurementReview<'a> {
    policies: Policies<'a>,
    times: TestedTimes<'a>,
    reconstruction: ReconstructionSamples<'a>,
    progress: ReviewProgress,
}
impl<'a> MeasurementReview<'a> {
    /// Preflight the record count, require a strict nested time refinement, and bound attempts.
    /// Caller-owned observations and stored output findings need their own storage reservations.
    pub fn new(
        policies: Policies<'a>,
        time_sets: [TestedTimes<'a>; 3],
        reconstruction: ReconstructionSamples<'a>,
        maximum_attempts: usize,
    ) -> Result<Self, VerificationError> {
        if !time_sets[1].refines(time_sets[0]) || !time_sets[2].refines(time_sets[1]) {
            return Err(VerificationError::InvalidTimes);
        }
        let times = time_sets[2];
        if !reconstruction.matches_times(times) {
            return Err(VerificationError::InvalidTimes);
        }
        let required = required_records(policies.as_slice().len(), times.as_slice().len())?;
        if maximum_attempts < required {
            return Err(VerificationError::CapacityExceeded);
        }
        Ok(Self {
            policies,
            times,
            reconstruction,
            progress: ReviewProgress {
                reviewed: 0,
                required,
                failed: 0,
                attempts_left: maximum_attempts,
            },
        })
    }

    /// Next time/observable pair, with the observable index varying fastest.
    /// A remaining position does not override an exhausted attempt allowance.
    pub fn next(&self) -> Option<(TickClock, u32)> {
        if self.progress.reviewed == self.progress.required {
            return None;
        }
        let entries = self.policies.as_slice();
        Some((
            self.times.as_slice()[self.progress.reviewed / entries.len()],
            entries[self.progress.reviewed % entries.len()].key,
        ))
    }

    /// Malformed inputs preserve all findings and the next position, but consume bounded work.
    /// A valid record with failed checks advances the schedule and remains a permanent failure.
    pub fn push(
        &mut self,
        input: Observation<'_>,
    ) -> Result<ObservationFindings, VerificationError> {
        if self.progress.attempts_left == 0 {
            return Err(VerificationError::CapacityExceeded);
        }
        self.progress.attempts_left -= 1;
        if self.next() != Some((input.time, input.key)) {
            return Err(VerificationError::UnexpectedObservation);
        }
        if input.comparison != ComparisonScope::FullField {
            return Err(VerificationError::InvalidComparisonScope);
        }
        let index = self.progress.reviewed % self.policies.as_slice().len();
        let findings = observation::evaluate(self.policies.as_slice()[index].budget, input)?;
        self.progress.failed += usize::from(!findings.passes());
        self.progress.reviewed += 1;
        Ok(findings)
    }

    /// Complete counters, including failures preceding missing/malformed records.
    pub fn progress(&self) -> ReviewProgress {
        self.progress
    }

    /// Retained exact off-stage geometry for provenance validation and the final report.
    pub fn reconstruction(&self) -> ReconstructionSamples<'a> {
        self.reconstruction
    }

    /// Numerical readiness is explicitly separate from a PDE acceptance decision.
    pub fn status(&self) -> ReviewStatus {
        if self.progress.reviewed < self.progress.required {
            if self.progress.attempts_left == 0 {
                ReviewStatus::BudgetExceeded
            } else {
                ReviewStatus::Incomplete
            }
        } else if self.progress.failed > 0 {
            ReviewStatus::Rejected
        } else {
            ReviewStatus::ReadyForLineageReview
        }
    }
}

/// Preflight the Cartesian record count before constructing an experiment's buffers.
pub fn required_records(observables: usize, samples: usize) -> Result<usize, VerificationError> {
    if observables == 0 || samples < 2 {
        return Err(VerificationError::InvalidValue);
    }
    observables
        .checked_mul(samples)
        .ok_or(VerificationError::CapacityExceeded)
}
