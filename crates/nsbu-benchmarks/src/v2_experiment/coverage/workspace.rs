use super::{
    CoverageFamilyPlan, CoverageFamilySample, CoverageFamilyWork, CoverageSamplingMetadata,
};
use crate::{
    regions::{NominalRegion, SpatialRegion},
    v2_experiment::{
        reference::{regional::RegionalTrackingSample, QUANTITIES},
        FamilyError, V2Family,
    },
};
use nsbu_solver::{diagnostics::local::SampledError, domain::TickClock, SolverError};

/// Coverage measurement refusal; no report is published after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverageFamilyError {
    /// The family, clock, identity, labels, or sampled metadata was invalid.
    Family(FamilyError),
    /// A bounded work or arithmetic operation refused.
    Numerical(SolverError),
}
impl From<FamilyError> for CoverageFamilyError {
    fn from(value: FamilyError) -> Self {
        Self::Family(value)
    }
}
impl From<SolverError> for CoverageFamilyError {
    fn from(value: SolverError) -> Self {
        Self::Numerical(value)
    }
}

/// Constant-storage coverage consumer borrowing immutable actual family state.
pub struct CoverageFamilyWorkspace<'a> {
    plan: CoverageFamilyPlan<'a>,
    charged: CoverageFamilyWork,
    next: usize,
    failed: bool,
    last: Option<CoverageFamilySample>,
}
impl<'a> CoverageFamilyWorkspace<'a> {
    /// Construct after the separate regional workspace and this consumer have been jointly budgeted by the caller.
    pub fn new(plan: CoverageFamilyPlan<'a>) -> Self {
        Self {
            plan,
            charged: CoverageFamilyWork::default(),
            next: 0,
            failed: false,
            last: None,
        }
    }
    /// Remaining attempt allowance.
    pub fn remaining(&self) -> usize {
        self.plan.bounds().work.attempts - self.charged.attempts
    }
    /// Work already charged, including a refused call.
    pub fn charged_work(&self) -> CoverageFamilyWork {
        self.charged
    }
    /// Last completely published report; failed calls never replace it.
    pub fn last_report(&self) -> Option<CoverageFamilySample> {
        self.last
    }
    /// Next exact accepted clock required by this consumer.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times().as_slice().get(self.next).copied()
    }
    /// Measure the next actual accepted regional report without changing its family or scratch.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
        regional: RegionalTrackingSample,
    ) -> Result<CoverageFamilySample, CoverageFamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated.into());
        }
        self.charge()?;
        let result = self.compute(family, regional);
        match result {
            Ok(sample) => {
                self.next += 1;
                self.last = Some(sample);
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.attempts += 1;
        self.charged.geometry_evaluations += self.plan.per_attempt.geometry_evaluations;
        self.charged.metadata_visits += self.plan.per_attempt.metadata_visits;
        Ok(())
    }
    fn compute(
        &self,
        family: &V2Family<'_>,
        regional: RegionalTrackingSample,
    ) -> Result<CoverageFamilySample, CoverageFamilyError> {
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        if family.plan().identity() != self.plan.family.identity()
            || regional.clock() != clock
            || regional.identity() != self.plan.family.identity()
            || (0..6).any(|index| {
                family
                    .branch(index)
                    .is_none_or(|run| run.state().clock() != clock)
            })
        {
            return Err(FamilyError::InvalidFamily.into());
        }
        let sampling = metadata(regional, clock)?;
        let core = self
            .plan
            .coverage
            .map(|plan| plan.evaluate(clock, NominalRegion::CORE));
        let annulus = self
            .plan
            .coverage
            .map(|plan| plan.evaluate(clock, NominalRegion::ANNULUS));
        let core = [
            core[0].map_err(|_| FamilyError::InvalidFamily)?,
            core[1].map_err(|_| FamilyError::InvalidFamily)?,
            core[2].map_err(|_| FamilyError::InvalidFamily)?,
        ];
        let annulus = [
            annulus[0].map_err(|_| FamilyError::InvalidFamily)?,
            annulus[1].map_err(|_| FamilyError::InvalidFamily)?,
            annulus[2].map_err(|_| FamilyError::InvalidFamily)?,
        ];
        Ok(CoverageFamilySample::from_parts(
            clock,
            self.plan.family.identity(),
            regional,
            core,
            annulus,
            sampling,
        ))
    }
}
fn metadata(
    regional: RegionalTrackingSample,
    clock: TickClock,
) -> Result<CoverageSamplingMetadata, FamilyError> {
    let layout = regional.sample_layout();
    let points = layout.real_len();
    let mut core = [None; 24];
    let mut annulus = [None; 24];
    for (branch_index, branch) in regional.branches().iter().enumerate() {
        if branch.branch != branch_index {
            return Err(FamilyError::InvalidFamily);
        }
        for (quantity_index, quantity) in branch.quantities.iter().enumerate() {
            let report = quantity.regional;
            if quantity.quantity != QUANTITIES[quantity_index]
                || report.clock != clock
                || report.dimensions != layout.dimensions()
                || !report.grid_complete
                || report.components != QUANTITIES[quantity_index].components()
                || report.global != SampledError::Measured(quantity.global)
                || quantity.global.samples != points
            {
                return Err(FamilyError::InvalidFamily);
            }
            let mut count = 0usize;
            let mut sampled_core = None;
            let mut sampled_annulus = None;
            for (region, value) in report.regions {
                let samples = sample_count(value);
                count = count
                    .checked_add(samples.unwrap_or(0))
                    .ok_or(SolverError::SizeOverflow)?;
                if region == SpatialRegion::Core {
                    sampled_core = samples;
                }
                if region == SpatialRegion::Annulus {
                    sampled_annulus = samples;
                }
            }
            if count != points {
                return Err(FamilyError::InvalidFamily);
            }
            let slot = branch_index * 4 + quantity_index;
            core[slot] = sampled_core;
            annulus[slot] = sampled_annulus;
        }
    }
    Ok(CoverageSamplingMetadata {
        layout,
        points,
        sampled_core: core,
        sampled_annulus: annulus,
    })
}

fn sample_count(value: SampledError) -> Option<usize> {
    match value {
        SampledError::Measured(value) => Some(value.samples),
        SampledError::NoSamples => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sample_count;
    use nsbu_solver::diagnostics::local::SampledError;

    #[test]
    fn no_samples_is_metadata_absence_not_a_zero_count() {
        assert_eq!(sample_count(SampledError::NoSamples), None);
    }
}
