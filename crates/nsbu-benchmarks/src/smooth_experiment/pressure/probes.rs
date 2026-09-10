//! Physical pressure at the probe time, from reconstructed velocity and a fresh prescribed force.
use super::{refinement, PressureFamilyWorkspace};
use crate::smooth_experiment::{
    physical::QuantityRefinement, probes::ProbeFamily, FamilyError, PAIRS,
};
use nsbu_solver::diagnostics::{local::LocalError, physical::PhysicalQuantity};

impl PressureFamilyWorkspace<'_> {
    // The owning probe consumer binds every field before this private numerical operation.
    // No accepted-time schedule is advanced here; its finite work ledger is still charged.
    pub(in crate::smooth_experiment) fn probe_quantities(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<[QuantityRefinement; 2], FamilyError> {
        self.charge()?;
        let clock = family.fields(0).ok_or(FamilyError::InvalidFamily)?.clock;
        self.evaluate_force(clock)?;
        let pair = |indices| self.probe_pair(family, indices);
        let [a, b, c, d, e] = PAIRS.map(pair);
        let pairs = [a?, b?, c?, d?, e?];
        Ok([
            refinement(PhysicalQuantity::Scalar, pairs, 0),
            refinement(PhysicalQuantity::ScalarGradient, pairs, 1),
        ])
    }
    fn probe_pair(
        &mut self,
        family: &ProbeFamily<'_>,
        (a, b): (usize, usize),
    ) -> Result<[LocalError; 2], FamilyError> {
        let left = family.fields(a).ok_or(FamilyError::InvalidFamily)?;
        let right = family.fields(b).ok_or(FamilyError::InvalidFamily)?;
        self.construct_values(left.domain, left.value, 0)?;
        self.construct_values(right.domain, right.value, 1)?;
        Ok([
            self.compare(PhysicalQuantity::Scalar, 0)?,
            self.compare(PhysicalQuantity::ScalarGradient, 1)?,
        ])
    }
}
