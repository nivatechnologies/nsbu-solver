//! Read-only analytical velocity/derivative tracking for all six exact-v2 family branches.
mod plan;
pub mod regional;
mod report;
use super::{FamilyError, V2Family};
use crate::{
    fields::reference::{self, ReferenceEvaluation},
    time::BenchmarkTime,
};
use nsbu_solver::{
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
        physical::PhysicalQuantity,
    },
    domain::{SpectralState, TickClock},
    SolverError,
};
pub use plan::{ReferenceTrackingBounds, ReferenceTrackingPlan, ReferenceTrackingWork};
pub use report::{BranchTracking, ReferenceTrackingSample, TrackingQuantity};

/// Fixed velocity, complete gradient, ordered Hessian and physical curl order.
pub const QUANTITIES: [PhysicalQuantity; 4] = [
    PhysicalQuantity::Vector,
    PhysicalQuantity::Gradient,
    PhysicalQuantity::Hessian,
    PhysicalQuantity::Vorticity,
];

/// Tracking-specific failures preserve their numerical, reference or family origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReferenceTrackingError {
    /// The borrowed family identity, clock, schedule or terminal state was invalid.
    Family(FamilyError),
    /// Analytical binary64 reference or bounded root evaluation failed.
    Reference(crate::BenchmarkError),
    /// Diagnostic allocation, FFT, spectrum or work admission failed.
    Numerical(SolverError),
    /// A prior numerical/reference computation failed before complete publication.
    Terminated,
}
impl From<FamilyError> for ReferenceTrackingError {
    fn from(error: FamilyError) -> Self {
        Self::Family(error)
    }
}
impl From<crate::BenchmarkError> for ReferenceTrackingError {
    fn from(error: crate::BenchmarkError) -> Self {
        Self::Reference(error)
    }
}
impl From<SolverError> for ReferenceTrackingError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

/// One cached analytical grid and sequential actual-state derivative scratch.
pub struct ReferenceTrackingWorkspace<'a> {
    plan: ReferenceTrackingPlan<'a>,
    derivatives: [DerivativeWorkspace; 3],
    actual: Vec<f64>,
    error_magnitudes: Vec<f64>,
    reference_magnitudes: Vec<f64>,
    references: Vec<ReferenceEvaluation>,
    charged: ReferenceTrackingWork,
    next: usize,
    failed: bool,
}
impl<'a> ReferenceTrackingWorkspace<'a> {
    /// Allocate the jointly admitted consumer without evaluating a reference or state.
    pub fn new(plan: ReferenceTrackingPlan<'a>) -> Result<Self, SolverError> {
        let sources = [0, 1, 2].map(|index| plan.family.branches[index].resources().domain());
        let count = plan.samples.real_len();
        Ok(Self {
            derivatives: [
                DerivativeWorkspace::new(sources[0], plan.samples, plan.bounds.storage_bytes)?,
                DerivativeWorkspace::new(sources[1], plan.samples, plan.bounds.storage_bytes)?,
                DerivativeWorkspace::new(sources[2], plan.samples, plan.bounds.storage_bytes)?,
            ],
            actual: filled(count, 0.0)?,
            error_magnitudes: filled(count, 0.0)?,
            reference_magnitudes: filled(count, 0.0)?,
            references: filled(count, zero_reference())?,
            plan,
            charged: ReferenceTrackingWork::default(),
            next: 0,
            failed: false,
        })
    }
    /// Remaining admitted report attempts; numerical termination cannot refresh them.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Worst-case work charged for all attempted reports, including refusals.
    pub fn charged_work(&self) -> ReferenceTrackingWork {
        self.charged
    }
    /// Next required exact family clock.
    pub fn next_time(&self) -> Option<TickClock> {
        self.plan.family.times.as_slice().get(self.next).copied()
    }
    /// True after a numerical computation failed before complete publication.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Measure every branch and quantity before publishing one immutable report.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<ReferenceTrackingSample, ReferenceTrackingError> {
        self.charge()?;
        let clock = self.plan.family.require_sample(family, self.next)?;
        match self.compute(family, clock) {
            Ok(sample) => {
                self.next += 1;
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn charge(&mut self) -> Result<(), ReferenceTrackingError> {
        if self.failed {
            return Err(ReferenceTrackingError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.reference_evaluations += self.plan.per_attempt.reference_evaluations;
        self.charged.root_iterations += self.plan.per_attempt.root_iterations;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &V2Family<'_>,
        clock: TickClock,
    ) -> Result<ReferenceTrackingSample, ReferenceTrackingError> {
        self.evaluate_references(clock)?;
        let branches = [
            self.branch(family, 0)?,
            self.branch(family, 1)?,
            self.branch(family, 2)?,
            self.branch(family, 3)?,
            self.branch(family, 4)?,
            self.branch(family, 5)?,
        ];
        Ok(ReferenceTrackingSample {
            clock,
            identity: self.plan.family.identity(),
            samples: self.plan.samples,
            floors: self.plan.floors,
            branches,
        })
    }
    fn evaluate_references(&mut self, clock: TickClock) -> Result<(), ReferenceTrackingError> {
        let time = BenchmarkTime::new(clock)?;
        let [nx, ny, nz] = self.plan.samples.dimensions();
        for (index, output) in self.references.iter_mut().enumerate() {
            let point = [
                (index / (ny * nz)) as f64 / nx as f64,
                ((index / nz) % ny) as f64 / ny as f64,
                (index % nz) as f64 / nz as f64,
            ];
            *output = reference::evaluate(point, time)?;
        }
        Ok(())
    }
    fn branch(
        &mut self,
        family: &V2Family<'_>,
        index: usize,
    ) -> Result<BranchTracking, ReferenceTrackingError> {
        let state = family.branches[index].state();
        Ok(BranchTracking {
            branch: index,
            quantities: [
                self.quantity(state, index, PhysicalQuantity::Vector, self.plan.floors[0])?,
                self.quantity(
                    state,
                    index,
                    PhysicalQuantity::Gradient,
                    self.plan.floors[1],
                )?,
                self.quantity(state, index, PhysicalQuantity::Hessian, self.plan.floors[2])?,
                self.quantity(
                    state,
                    index,
                    PhysicalQuantity::Vorticity,
                    self.plan.floors[3],
                )?,
            ],
        })
    }
    fn quantity(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
        floor: f64,
    ) -> Result<TrackingQuantity, ReferenceTrackingError> {
        self.prepare_quantity(state, branch, quantity)?;
        Ok(TrackingQuantity {
            quantity,
            error: self.reduce(quantity.components(), floor)?,
        })
    }
    fn prepare_quantity(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
    ) -> Result<(), ReferenceTrackingError> {
        self.error_magnitudes.fill(0.0);
        self.reference_magnitudes.fill(0.0);
        for component in 0..quantity.components() {
            self.sample_component(state, branch, quantity, component)?;
            for index in 0..self.actual.len() {
                let expected = reference_component(self.references[index], quantity, component);
                let difference = self.actual[index] - expected;
                self.error_magnitudes[index] =
                    accumulate_magnitude(self.error_magnitudes[index], difference)?;
                self.reference_magnitudes[index] =
                    accumulate_magnitude(self.reference_magnitudes[index], expected)?;
            }
        }
        Ok(())
    }
    fn sample_component(
        &mut self,
        state: &SpectralState,
        branch: usize,
        quantity: PhysicalQuantity,
        index: usize,
    ) -> Result<(), SolverError> {
        let workspace = if branch < 2 { branch } else { 2 };
        let (component, derivative) = entry(quantity, index);
        self.actual.copy_from_slice(
            self.derivatives[workspace]
                .sample(state.component(component)?, Derivative::new(derivative)?)?
                .values,
        );
        if quantity == PhysicalQuantity::Vorticity {
            let mut other = [0; 3];
            other[(index + 2) % 3] = 1;
            let values = self.derivatives[workspace]
                .sample(state.component((index + 1) % 3)?, Derivative::new(other)?)?;
            for (actual, &second) in self.actual.iter_mut().zip(values.values) {
                *actual -= second;
            }
        }
        Ok(())
    }
    fn reduce(&self, components: usize, floor: f64) -> Result<LocalError, SolverError> {
        match components {
            3 => reduce::<3>(&self.error_magnitudes, &self.reference_magnitudes, floor),
            9 => reduce::<9>(&self.error_magnitudes, &self.reference_magnitudes, floor),
            27 => reduce::<27>(&self.error_magnitudes, &self.reference_magnitudes, floor),
            _ => Err(SolverError::InvalidPayload),
        }
    }
}

fn reduce<const C: usize>(
    errors: &[f64],
    references: &[f64],
    floor: f64,
) -> Result<LocalError, SolverError> {
    let mut result = TensorErrors::<C>::new(errors.len(), floor)?;
    for (&error, &reference) in errors.iter().zip(references) {
        result.push_magnitudes(error, reference)?;
    }
    match result.finish()? {
        SampledError::Measured(value) => Ok(value),
        SampledError::NoSamples => Err(SolverError::InvalidPayload),
    }
}

fn entry(quantity: PhysicalQuantity, index: usize) -> (usize, [u8; 3]) {
    let mut orders = [0; 3];
    let component = match quantity {
        PhysicalQuantity::Vector => index,
        PhysicalQuantity::Gradient => {
            orders[index % 3] = 1;
            index / 3
        }
        PhysicalQuantity::Hessian => {
            orders[(index / 3) % 3] += 1;
            orders[index % 3] += 1;
            index / 9
        }
        PhysicalQuantity::Vorticity => {
            orders[(index + 1) % 3] = 1;
            (index + 2) % 3
        }
        _ => 0,
    };
    (component, orders)
}

fn reference_component(
    value: ReferenceEvaluation,
    quantity: PhysicalQuantity,
    index: usize,
) -> f64 {
    match quantity {
        PhysicalQuantity::Vector => value.velocity[index],
        PhysicalQuantity::Gradient => value.gradient[index / 3][index % 3],
        PhysicalQuantity::Hessian => value.hessian[index / 9][(index / 3) % 3][index % 3],
        PhysicalQuantity::Vorticity => value.vorticity[index],
        _ => f64::NAN,
    }
}

fn accumulate_magnitude(current: f64, component: f64) -> Result<f64, SolverError> {
    let result = current.hypot(component);
    if result.is_finite() {
        Ok(result)
    } else {
        Err(SolverError::InvalidSpectrum)
    }
}

fn zero_reference() -> ReferenceEvaluation {
    ReferenceEvaluation {
        velocity: [0.0; 3],
        gradient: [[0.0; 3]; 3],
        hessian: [[[0.0; 3]; 3]; 3],
        vorticity: [0.0; 3],
        pressure_raw: 0.0,
        pressure_gradient: [0.0; 3],
        root: None,
    }
}

fn filled<T: Clone>(count: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(count, value);
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::accumulate_magnitude;

    #[test]
    fn component_magnitudes_preserve_tiny_values_and_avoid_intermediate_square_overflow() {
        let tiny = accumulate_magnitude(0.0, 1e-200).unwrap();
        assert!(tiny > 0.0);
        let large = accumulate_magnitude(1e200, 1e200).unwrap();
        assert!(large.is_finite());
        assert!((large / 1e200 - 2.0_f64.sqrt()).abs() < 1e-15);
        assert!(accumulate_magnitude(f64::MAX, f64::MAX).is_err());
    }
}
