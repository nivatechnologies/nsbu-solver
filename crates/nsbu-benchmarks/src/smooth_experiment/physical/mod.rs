//! Complete vector/derivative refinements from the family's six actual rest trajectories.
mod plan;
mod probes;
use super::{FamilyError, SmoothFamily, PAIRS};
use nsbu_solver::{
    diagnostics::{
        local::LocalError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Layout, TickClock},
    SolverError,
};
pub use plan::{PhysicalFamilyBounds, PhysicalFamilyPlan, PhysicalFamilyWork};

/// Fixed ordered inventory for this vector-field increment. Pressure remains a separate study.
pub const QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

/// Five internally produced full-field comparisons for one complete physical quantity.
#[derive(Debug, Clone, Copy)]
pub struct QuantityRefinement {
    /// Exact field/ordered derivative inventory.
    pub quantity: PhysicalQuantity,
    /// N0/N1 and N1/N2 differences, retaining every fine mode.
    pub space: [LocalError; 2],
    /// H0/H1 and H1/H2 differences on the finest retained grid.
    pub time: [LocalError; 2],
    /// Independent CM/HO difference on the finest grid/smallest macro step.
    pub method: LocalError,
}
/// Complete internally computed physical findings at one exact synchronized accepted clock.
/// This type cannot establish the missing reference/force/arithmetic/pressure channels.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalRefinementSample {
    clock: TickClock,
    samples: Layout,
    quantities: [QuantityRefinement; 4],
}
impl PhysicalRefinementSample {
    /// Actual clock reached independently by all six owned branches.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Unaligned physical grid used by every quantity and every comparison pair.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed velocity/gradient/Hessian/vorticity inventory, with every mandatory pair present.
    pub fn quantities(&self) -> &[QuantityRefinement; 4] {
        &self.quantities
    }
}

/// One reused physical comparison workspace and a finite schedule/work ledger.
/// The family stays borrowed during measurement; no analytical reference or state assignment exists.
pub struct PhysicalFamilyWorkspace<'a> {
    plan: PhysicalFamilyPlan<'a>,
    workspace: PhysicalComparisonWorkspace,
    charged: PhysicalFamilyWork,
    next: usize,
}
impl<'a> PhysicalFamilyWorkspace<'a> {
    /// Allocate only the already jointly admitted diagnostic storage.
    pub fn new(plan: PhysicalFamilyPlan<'a>) -> Result<Self, SolverError> {
        let domain = plan.family.branches[2].plan.resources().domain();
        Ok(Self {
            plan,
            workspace: PhysicalComparisonWorkspace::new(
                domain,
                domain,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            charged: PhysicalFamilyWork::default(),
            next: 0,
        })
    }
    /// Unspent call allowance; malformed calls cannot obtain new attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Worst-case work spent, including unsuccessful calls before numerical traversal.
    pub fn charged_work(&self) -> PhysicalFamilyWork {
        self.charged
    }
    /// Next required physical clock; an exhausted allowance can still have missing evidence.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Measure only the next scheduled time after the matching family advance succeeds.
    /// Failed calls spend their full allowance, retain the schedule position and issue no partial report.
    pub fn measure(
        &mut self,
        family: &SmoothFamily<'_>,
    ) -> Result<PhysicalRefinementSample, FamilyError> {
        self.charge()?;
        let clock = self.next_time().ok_or(FamilyError::InvalidFamily)?;
        self.plan.family.require_sample(family, self.next)?;
        let quantities = [
            self.quantity(family, 0)?,
            self.quantity(family, 1)?,
            self.quantity(family, 2)?,
            self.quantity(family, 3)?,
        ];
        self.next += 1;
        Ok(PhysicalRefinementSample {
            clock,
            samples: self.plan.samples,
            quantities,
        })
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        // The preflight checked each complete allowance product before allocation.
        self.charged.attempts += 1;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn quantity(
        &mut self,
        family: &SmoothFamily<'_>,
        index: usize,
    ) -> Result<QuantityRefinement, FamilyError> {
        let quantity = QUANTITIES[index];
        let floor = self.plan.floors[index];
        let findings = [
            self.pair(family, PAIRS[0], quantity, floor)?,
            self.pair(family, PAIRS[1], quantity, floor)?,
            self.pair(family, PAIRS[2], quantity, floor)?,
            self.pair(family, PAIRS[3], quantity, floor)?,
            self.pair(family, PAIRS[4], quantity, floor)?,
        ];
        Ok(QuantityRefinement {
            quantity,
            space: [findings[0], findings[1]],
            time: [findings[2], findings[3]],
            method: findings[4],
        })
    }
    fn pair(
        &mut self,
        family: &SmoothFamily<'_>,
        pair: (usize, usize),
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<LocalError, FamilyError> {
        let left = family.branches[pair.0].state();
        let right = family.branches[pair.1].state();
        Ok(self
            .workspace
            .compare_domains(
                [left.plan().domain(), right.plan().domain()],
                field(left)?,
                field(right)?,
                quantity,
                floor,
            )?
            .global())
    }
}

fn field(state: &nsbu_solver::domain::SpectralState) -> Result<PhysicalField<'_>, SolverError> {
    Ok(PhysicalField::Vector([
        state.component(0)?,
        state.component(1)?,
        state.component(2)?,
    ]))
}
