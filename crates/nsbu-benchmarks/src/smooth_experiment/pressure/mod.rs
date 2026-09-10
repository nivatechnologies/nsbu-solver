//! Independent full-quadratic-band pressure from each actual accepted family branch.
mod plan;
#[cfg(test)]
mod tests;
use super::{physical::QuantityRefinement, FamilyError, SmoothFamily, PAIRS};
use crate::smooth::CyclicSine;
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace,
        local::LocalError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Domain, Layout, SpectralState, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::transfer,
    Complex64, SolverError,
};
pub use plan::{PressureFamilyBounds, PressureFamilyPlan, PressureFamilyWork};
type Field = [Vec<Complex64>; 3];

/// Complete pressure and pressure-gradient findings at one synchronized accepted clock.
/// Mean zero is the global physical gauge; no regional or fitted mean is removed.
#[derive(Debug, Clone, Copy)]
pub struct PressureRefinementSample {
    clock: TickClock,
    source: Domain,
    samples: Layout,
    quantities: [QuantityRefinement; 2],
}
impl PressureRefinementSample {
    /// Actual clock reached by all six independently evolved states.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Common finest velocity domain; pressures retain twice each of these axes.
    pub fn source_domain(self) -> Domain {
        self.source
    }
    /// Unaligned physical lattice shared by both pressure quantities.
    pub fn sample_layout(self) -> Layout {
        self.samples
    }
    /// Scalar pressure followed by its complete three-component physical gradient.
    pub fn quantities(&self) -> &[QuantityRefinement; 2] {
        &self.quantities
    }
}

/// Independent prescribed force, conservative products and reused scalar comparison scratch.
/// No stage RHS, analytical pressure or mutable integrated state is received.
pub struct PressureFamilyWorkspace<'a> {
    plan: PressureFamilyPlan<'a>,
    products: ConservativeWorkspace,
    comparison: PhysicalComparisonWorkspace,
    provider: CyclicSine,
    provider_limit: ForceLimits,
    velocity: Field,
    force: Field,
    conservative: Field,
    pressure: [Vec<Complex64>; 2],
    spent: PressureFamilyWork,
    next: usize,
}
impl<'a> PressureFamilyWorkspace<'a> {
    /// Construct only after the family and full independent diagnostic storage were admitted.
    pub fn new(plan: PressureFamilyPlan<'a>) -> Result<Self, SolverError> {
        let source = plan.source;
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let provider = CyclicSine::new(diagnostic)?;
        let provider_limit = provider.limits().ok_or(SolverError::UnknownProviderCost)?;
        let n = source.layout().half_len();
        let m = diagnostic.layout().half_len();
        Ok(Self {
            plan,
            products: ConservativeWorkspace::new(source, plan.bounds.storage_bytes)?,
            comparison: PhysicalComparisonWorkspace::new(
                diagnostic,
                diagnostic,
                plan.samples,
                plan.bounds.storage_bytes,
            )?,
            provider,
            provider_limit,
            velocity: field(n)?,
            force: field(m)?,
            conservative: field(m)?,
            pressure: [filled(m)?, filled(m)?],
            spent: PressureFamilyWork::default(),
            next: 0,
        })
    }
    /// Remaining requests; malformed requests also spend one complete allowance.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.maximum_attempts - self.spent.attempts
    }
    /// Conservative work charge retained through every failure.
    pub fn charged_work(&self) -> PressureFamilyWork {
        self.spent
    }
    /// Next required clock; no missing sample can be silently skipped.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// Form all five pressure/gradient pairs from synchronized accepted states.
    /// Each side is reconstructed independently on the complete common doubled band.
    /// Failure spends work, leaves the schedule and all trajectories unchanged, and issues no report.
    pub fn measure(
        &mut self,
        family: &SmoothFamily<'_>,
    ) -> Result<PressureRefinementSample, FamilyError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        self.evaluate_force(clock)?;
        let pairs = [
            self.pair(family, PAIRS[0])?,
            self.pair(family, PAIRS[1])?,
            self.pair(family, PAIRS[2])?,
            self.pair(family, PAIRS[3])?,
            self.pair(family, PAIRS[4])?,
        ];
        self.next += 1;
        Ok(PressureRefinementSample {
            clock,
            source: self.plan.source,
            samples: self.plan.samples,
            quantities: [
                refinement(PhysicalQuantity::Scalar, pairs, 0),
                refinement(PhysicalQuantity::ScalarGradient, pairs, 1),
            ],
        })
    }
    // Provider cost/assembly is distinct from complete report scheduling and publication.
    fn evaluate_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        let work = self.provider.evaluate(
            clock,
            self.provider_limit,
            self.force.each_mut().map(Vec::as_mut_slice),
        )?;
        if work.work_units > self.provider_limit.work_units
            || work.scalar_transforms > self.provider_limit.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        // Admission checked every complete product before allocation; all charges are retained.
        self.spent.attempts += 1;
        self.spent.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.spent.provider_work_units += self.plan.per_attempt.provider_work_units;
        self.spent.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn pair(
        &mut self,
        family: &SmoothFamily<'_>,
        (a, b): (usize, usize),
    ) -> Result<[LocalError; 2], FamilyError> {
        self.construct(family.branches[a].state(), 0)?;
        self.construct(family.branches[b].state(), 1)?;
        Ok([
            self.compare(PhysicalQuantity::Scalar, 0)?,
            self.compare(PhysicalQuantity::ScalarGradient, 1)?,
        ])
    }
    fn compare(
        &mut self,
        quantity: PhysicalQuantity,
        index: usize,
    ) -> Result<LocalError, SolverError> {
        Ok(self
            .comparison
            .compare(
                PhysicalField::Scalar(&self.pressure[0]),
                PhysicalField::Scalar(&self.pressure[1]),
                quantity,
                self.plan.floors[index],
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
    // Kept private: only the actual family binding may submit production state fields.
    fn construct_values(
        &mut self,
        domain: Domain,
        values: [&[Complex64]; 3],
        side: usize,
    ) -> Result<(), SolverError> {
        for (axis, values) in values.into_iter().enumerate() {
            transfer(
                domain.layout(),
                self.plan.source.layout(),
                values,
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
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(n, Complex64::new(0.0, 0.0));
    Ok(values)
}
