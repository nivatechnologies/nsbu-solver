//! Complete three-consumer computation before aggregate report publication.
use super::{SamplingQuantity, SamplingSample, SamplingWorkspace};
use crate::v2_experiment::{FamilyError, V2Family};
use nsbu_solver::domain::TickClock;

impl SamplingWorkspace<'_> {
    pub(super) fn compute(
        &mut self,
        family: &V2Family<'_>,
        clock: TickClock,
    ) -> Result<SamplingSample, FamilyError> {
        let levels = [
            self.physical[0].measure(family)?,
            self.physical[1].measure(family)?,
            self.physical[2].measure(family)?,
        ];
        Ok(SamplingSample {
            clock,
            identity: self.plan.family.identity(),
            layouts: self.plan.sample_layouts(),
            floors: self.plan.relative_floors(),
            quantities: std::array::from_fn(|index| SamplingQuantity {
                levels: levels.map(|level| level.quantities()[index]),
            }),
        })
    }
}
