//! Private reuse of complete physical sampling after the outer probe consumer binds provenance.
use super::{PhysicalFamilyWorkspace, QuantityRefinement, QUANTITIES};
use crate::smooth_experiment::{probes::ProbeFamily, FamilyError, PAIRS};
use nsbu_solver::diagnostics::{local::LocalError, physical::PhysicalField};

impl PhysicalFamilyWorkspace<'_> {
    // The owning probe consumer supplies the schedule; this child's accepted-time counter is unused.
    pub(in crate::smooth_experiment) fn probe_quantities(
        &mut self,
        family: &ProbeFamily<'_>,
    ) -> Result<[QuantityRefinement; 4], FamilyError> {
        self.charge()?;
        Ok([
            self.probe_quantity(family, 0)?,
            self.probe_quantity(family, 1)?,
            self.probe_quantity(family, 2)?,
            self.probe_quantity(family, 3)?,
        ])
    }
    fn probe_quantity(
        &mut self,
        family: &ProbeFamily<'_>,
        index: usize,
    ) -> Result<QuantityRefinement, FamilyError> {
        let compare = |pair| self.probe_pair(family, pair, index);
        let [a, b, c, d, e] = PAIRS.map(compare);
        Ok(QuantityRefinement {
            quantity: QUANTITIES[index],
            space: [a?, b?],
            time: [c?, d?],
            method: e?,
        })
    }
    fn probe_pair(
        &mut self,
        family: &ProbeFamily<'_>,
        (a, b): (usize, usize),
        index: usize,
    ) -> Result<LocalError, FamilyError> {
        let left = family.fields(a).ok_or(FamilyError::InvalidFamily)?;
        let right = family.fields(b).ok_or(FamilyError::InvalidFamily)?;
        Ok(self
            .workspace
            .compare_domains(
                [left.domain, right.domain],
                PhysicalField::Vector(left.value),
                PhysicalField::Vector(right.value),
                QUANTITIES[index],
                self.plan.floors[index],
            )?
            .global())
    }
}
