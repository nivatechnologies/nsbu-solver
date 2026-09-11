//! Complete physical refinements from actual exact-v2 accepted states.
mod plan;
use super::{FamilyError, V2Family, PAIRS};
use nsbu_solver::{
    diagnostics::{
        local::LocalError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Layout, SpectralState, TickClock},
    SolverError,
};
pub use plan::{PhysicalFamilyBounds, PhysicalFamilyPlan, PhysicalFamilyWork};

/// Fixed velocity, gradient, Hessian and vorticity order.
pub const QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

#[derive(Debug, Clone, Copy)]
/// Five complete pair findings for one physical quantity.
pub struct QuantityRefinement {
    /// Measured physical quantity.
    pub quantity: PhysicalQuantity,
    /// Spatial pairs `(N0,N1)` and `(N1,N2)`.
    pub space: [LocalError; 2],
    /// Temporal pairs `(H0,H1)` and `(H1,H2)`.
    pub time: [LocalError; 2],
    /// Finest-grid CM/HO pair.
    pub method: LocalError,
}

#[derive(Debug, Clone, Copy)]
/// Complete physical findings at one actual accepted V2 clock.
pub struct PhysicalRefinementSample {
    clock: TickClock,
    identity: [u8; 32],
    samples: Layout,
    floors: [f64; 4],
    quantities: [QuantityRefinement; 4],
}
impl PhysicalRefinementSample {
    /// Actual synchronized accepted clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Identity of the V2 family that produced this report.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Diagnostic physical sample layout.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Fixed relative-error floors in [`QUANTITIES`] order.
    pub fn relative_floors(self) -> [f64; 4] {
        self.floors
    }
    /// Four findings in [`QUANTITIES`] order.
    pub fn quantities(&self) -> &[QuantityRefinement; 4] {
        &self.quantities
    }
}

/// Reusable bounded physical comparison workspace for one V2 family.
pub struct PhysicalFamilyWorkspace<'a> {
    plan: PhysicalFamilyPlan<'a>,
    workspace: PhysicalComparisonWorkspace,
    charged: PhysicalFamilyWork,
    next: usize,
}
impl<'a> PhysicalFamilyWorkspace<'a> {
    /// Construct from an already jointly admitted plan.
    pub fn new(plan: PhysicalFamilyPlan<'a>) -> Result<Self, SolverError> {
        let domain = plan.family.branches[2].resources().domain();
        Ok(Self {
            workspace: PhysicalComparisonWorkspace::new(
                domain,
                domain,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            plan,
            charged: PhysicalFamilyWork::default(),
            next: 0,
        })
    }
    /// Remaining finite measurement attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.charged.attempts
    }
    /// Charged work, including failed calls.
    pub fn charged_work(&self) -> PhysicalFamilyWork {
        self.charged
    }
    /// Next required family clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Compare all quantities at the next accepted family clock.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
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
            identity: self.plan.family.identity(),
            samples: self.plan.samples,
            floors: self.plan.floors,
            quantities,
        })
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.charged.attempts += 1;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn quantity(
        &mut self,
        family: &V2Family<'_>,
        index: usize,
    ) -> Result<QuantityRefinement, FamilyError> {
        let quantity = QUANTITIES[index];
        let floor = self.plan.floors[index];
        let findings = PAIRS.map(|pair| self.pair(family, pair, quantity, floor));
        Ok(QuantityRefinement {
            quantity,
            space: [findings[0]?, findings[1]?],
            time: [findings[2]?, findings[3]?],
            method: findings[4]?,
        })
    }
    fn pair(
        &mut self,
        family: &V2Family<'_>,
        (a, b): (usize, usize),
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<LocalError, FamilyError> {
        let left = family.branches[a].state();
        let right = family.branches[b].state();
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

fn field(state: &SpectralState) -> Result<PhysicalField<'_>, SolverError> {
    Ok(PhysicalField::Vector([
        state.component(0)?,
        state.component(1)?,
        state.component(2)?,
    ]))
}
