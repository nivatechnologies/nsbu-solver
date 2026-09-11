//! Full doubled-band pressure findings from actual exact-v2 family states.
mod plan;
#[cfg(test)]
mod tests;
use super::{FamilyError, V2Family, PAIRS};
use crate::provider::V2Force;
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace,
        local::LocalError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Domain, Layout, SpectralState, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::transfer,
    Complex64, SolverError,
};
pub use plan::{PressureFamilyBounds, PressureFamilyPlan, PressureFamilyWork};
type Field = [Vec<Complex64>; 3];
/// Fixed pressure and pressure-gradient order.
pub const QUANTITIES: [PhysicalQuantity; 2] =
    [PhysicalQuantity::Scalar, PhysicalQuantity::ScalarGradient];
/// Five findings for one pressure quantity.
#[derive(Debug, Clone, Copy)]
pub struct QuantityRefinement {
    /// Quantity measured.
    pub quantity: PhysicalQuantity,
    /// Spatial pairs `(N0,N1)` and `(N1,N2)`.
    pub space: [LocalError; 2],
    /// Temporal pairs `(H0,H1)` and `(H1,H2)`.
    pub time: [LocalError; 2],
    /// CM/HO pair.
    pub method: LocalError,
}
/// Pressure and pressure-gradient findings for one accepted family clock.
#[derive(Debug, Clone, Copy)]
pub struct PressureRefinementSample {
    clock: TickClock,
    identity: [u8; 32],
    source: Domain,
    samples: Layout,
    force: Layout,
    floors: [f64; 2],
    quantities: [QuantityRefinement; 2],
}
impl PressureRefinementSample {
    /// Actual accepted synchronized clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// V2 family identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Finest source velocity domain.
    pub fn source_domain(self) -> Domain {
        self.source
    }
    /// Physical comparison sample layout.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Full doubled pressure/force layout.
    pub fn force_layout(self) -> Layout {
        self.force
    }
    /// Pressure and gradient relative floors.
    pub fn relative_floors(self) -> [f64; 2] {
        self.floors
    }
    /// Findings in scalar pressure, scalar-gradient order.
    pub fn quantities(&self) -> &[QuantityRefinement; 2] {
        &self.quantities
    }
}
/// Reusable bounded pressure construction and comparison workspace.
pub struct PressureFamilyWorkspace<'a> {
    plan: PressureFamilyPlan<'a>,
    products: ConservativeWorkspace,
    comparison: PhysicalComparisonWorkspace,
    provider: V2Force,
    velocity: Field,
    force: Field,
    conservative: Field,
    pressure: [Vec<Complex64>; 2],
    spent: PressureFamilyWork,
    next: usize,
}
impl<'a> PressureFamilyWorkspace<'a> {
    /// Construct after complete joint admission.
    pub fn new(plan: PressureFamilyPlan<'a>) -> Result<Self, SolverError> {
        let n = plan.source.layout().half_len();
        let m = plan.diagnostic.layout().half_len();
        Ok(Self {
            products: ConservativeWorkspace::new(plan.source, plan.bounds.storage_bytes)?,
            comparison: PhysicalComparisonWorkspace::new(
                plan.diagnostic,
                plan.diagnostic,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            provider: V2Force::new(
                plan.diagnostic,
                plan.diagnostic.layout(),
                plan.bounds.storage_bytes,
            )?,
            velocity: field(n)?,
            force: field(m)?,
            conservative: field(m)?,
            pressure: [filled(m)?, filled(m)?],
            spent: PressureFamilyWork::default(),
            next: 0,
            plan,
        })
    }
    /// Remaining finite attempts.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.spent.attempts
    }
    /// Charged work, including failed calls.
    pub fn charged_work(&self) -> PressureFamilyWork {
        self.spent
    }
    /// Next required accepted clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Construct and compare pressure and its complete physical gradient.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<PressureRefinementSample, FamilyError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        self.evaluate_force(clock)?;
        let pairs = PAIRS.map(|pair| self.pair(family, pair));
        let pairs = [pairs[0]?, pairs[1]?, pairs[2]?, pairs[3]?, pairs[4]?];
        self.next += 1;
        Ok(PressureRefinementSample {
            clock,
            identity: self.plan.family.identity(),
            source: self.plan.source,
            samples: self.plan.samples,
            force: self.plan.diagnostic.layout(),
            floors: self.plan.floors,
            quantities: [
                refinement(QUANTITIES[0], pairs, 0),
                refinement(QUANTITIES[1], pairs, 1),
            ],
        })
    }
    fn evaluate_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        let work = self.provider.evaluate(
            clock,
            self.plan.provider_limits,
            self.force.each_mut().map(Vec::as_mut_slice),
        )?;
        if work.work_units > self.plan.provider_limits.work_units
            || work.scalar_transforms > self.plan.provider_limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.spent.attempts += 1;
        self.spent.provider_work_units += self.plan.per_attempt.provider_work_units;
        self.spent.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.spent.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn pair(
        &mut self,
        family: &V2Family<'_>,
        (a, b): (usize, usize),
    ) -> Result<[LocalError; 2], FamilyError> {
        self.construct(family.branches[a].state(), 0)?;
        self.construct(family.branches[b].state(), 1)?;
        Ok([
            self.compare(PhysicalQuantity::Scalar, 0)?,
            self.compare(PhysicalQuantity::ScalarGradient, 1)?,
        ])
    }
    fn compare(&mut self, q: PhysicalQuantity, i: usize) -> Result<LocalError, SolverError> {
        Ok(self
            .comparison
            .compare(
                PhysicalField::Scalar(&self.pressure[0]),
                PhysicalField::Scalar(&self.pressure[1]),
                q,
                self.plan.floors[i],
            )?
            .global())
    }
    fn construct(&mut self, state: &SpectralState, side: usize) -> Result<(), SolverError> {
        self.construct_values(
            state.plan().domain(),
            [
                state.component(0)?,
                state.component(1)?,
                state.component(2)?,
            ],
            side,
        )
    }
    fn construct_values(
        &mut self,
        domain: Domain,
        values: [&[Complex64]; 3],
        side: usize,
    ) -> Result<(), SolverError> {
        for (axis, value) in values.into_iter().enumerate() {
            transfer(
                domain.layout(),
                self.plan.source.layout(),
                value,
                &mut self.velocity[axis],
            )?;
        }
        self.products.evaluate(
            self.velocity.each_ref().map(Vec::as_slice),
            self.force.each_ref().map(Vec::as_slice),
            self.conservative.each_mut().map(Vec::as_mut_slice),
            &mut self.pressure[side],
        )
    }
}
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    v.resize(n, Complex64::new(0.0, 0.0));
    Ok(v)
}
fn refinement(
    quantity: PhysicalQuantity,
    pairs: [[LocalError; 2]; 5],
    index: usize,
) -> QuantityRefinement {
    QuantityRefinement {
        quantity,
        space: [pairs[0][index], pairs[1][index]],
        time: [pairs[2][index], pairs[3][index]],
        method: pairs[4][index],
    }
}
