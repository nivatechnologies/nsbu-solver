//! Complete quantity and pair computations over borrowed family states.
use super::PhysicalFamilyWorkspace;
use crate::v2_experiment::{
    physical::{PhysicalExtrema, QuantityRefinement, QUANTITIES},
    FamilyError, V2Family, PAIRS,
};
use nsbu_solver::{
    diagnostics::{
        local::LocalError,
        physical::{PhysicalField, PhysicalQuantity},
    },
    domain::SpectralState,
    SolverError,
};

impl PhysicalFamilyWorkspace<'_> {
    pub(super) fn quantities(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<[QuantityRefinement; 4], FamilyError> {
        Ok([
            self.quantity(family, 0)?,
            self.quantity(family, 1)?,
            self.quantity(family, 2)?,
            self.quantity(family, 3)?,
        ])
    }
    fn quantity(
        &mut self,
        family: &V2Family<'_>,
        index: usize,
    ) -> Result<QuantityRefinement, FamilyError> {
        let quantity = QUANTITIES[index];
        let floor = self.plan.floors[index];
        let [a, b, c, d, e] = PAIRS.map(|pair| self.pair(family, pair, quantity, floor));
        let [a, b, c, d, e] = [a?, b?, c?, d?, e?];
        Ok(QuantityRefinement {
            quantity,
            space: [a.0, b.0],
            time: [c.0, d.0],
            method: e.0,
            extrema: [a.1, b.1, c.1, d.1, e.1],
        })
    }
    fn pair(
        &mut self,
        family: &V2Family<'_>,
        (a, b): (usize, usize),
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<(LocalError, PhysicalExtrema), FamilyError> {
        let left = family.branches[a].state();
        let right = family.branches[b].state();
        let comparison = self.workspace.compare_domains(
            [left.plan().domain(), right.plan().domain()],
            field(left)?,
            field(right)?,
            quantity,
            floor,
        )?;
        Ok((
            comparison.global(),
            PhysicalExtrema::from_comparison(&comparison, floor)?,
        ))
    }
}

fn field(state: &SpectralState) -> Result<PhysicalField<'_>, SolverError> {
    Ok(PhysicalField::Vector([
        state.component(0)?,
        state.component(1)?,
        state.component(2)?,
    ]))
}
