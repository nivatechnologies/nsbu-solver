//! Globally gauged analytical pressure tracking for actual accepted exact-v2 states.
mod gauge;
mod plan;
mod report;
use crate::{
    fields::reference::{self, ReferenceEvaluation},
    runtime_force::RunForce,
    time::BenchmarkTime,
    v2_experiment::{
        pressure::{construct_pressure, PressureScratch},
        FamilyError, V2Family,
    },
};
pub use gauge::{GaugeChanges, GaugeError, GaugeEstimate, ImportedGauge};
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace,
        derivatives::{Derivative, DerivativeWorkspace},
        local::{LocalError, SampledError, TensorErrors},
    },
    domain::{SpectralState, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
pub use plan::PressureReferencePlan;
pub use report::{BranchPressureTracking, PressureReferenceSample};
type Field = [Vec<Complex64>; 3];

/// Complete bounded numerical work, charged before every attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PressureReferenceWork {
    /// Complete report attempts.
    pub attempts: usize,
    /// Cached analytical evaluations.
    pub reference_evaluations: usize,
    /// Conservative analytical root-iteration allowance.
    pub root_iterations: usize,
    /// Original-force provider work units.
    pub provider_work_units: usize,
    /// Force and pressure sampling transforms.
    pub scalar_transforms: usize,
    /// Conservative coefficient, sample and reduction visits.
    pub weighted_visits: usize,
}
impl PressureReferenceWork {
    fn scale(self, count: usize) -> Result<Self, SolverError> {
        Ok(Self {
            attempts: count,
            reference_evaluations: mul(self.reference_evaluations, count)?,
            root_iterations: mul(self.root_iterations, count)?,
            provider_work_units: mul(self.provider_work_units, count)?,
            scalar_transforms: mul(self.scalar_transforms, count)?,
            weighted_visits: mul(self.weighted_visits, count)?,
        })
    }
}
/// Consumer-owned and joint-family storage plus complete work bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressureReferenceBounds {
    /// Conservative consumer storage admission.
    pub storage_bytes: usize,
    /// Six trajectories plus this consumer.
    pub joint_storage_bytes: usize,
    /// Complete three-clock work allowance.
    pub work: PressureReferenceWork,
    /// One-time caller import bytes already hashed before plan admission.
    pub imported_gauge_hash_bytes: usize,
}
/// Transactional all-six analytical pressure workspace.
pub struct PressureReferenceWorkspace<'a> {
    plan: PressureReferencePlan<'a>,
    products: ConservativeWorkspace,
    derivatives: DerivativeWorkspace,
    provider: RunForce,
    velocity: Field,
    force: Field,
    conservative: Field,
    pressure: Vec<Complex64>,
    actual: Vec<f64>,
    errors: Vec<f64>,
    references: Vec<f64>,
    analytical: Vec<ReferenceEvaluation>,
    charged: PressureReferenceWork,
    next: usize,
    failed: bool,
}
impl<'a> PressureReferenceWorkspace<'a> {
    /// Allocate only after complete joint admission; no state or reference is sampled.
    pub fn new(plan: PressureReferencePlan<'a>) -> Result<Self, SolverError> {
        let source = plan.pressure.source_domain();
        let diagnostic = plan.pressure.diagnostic_domain();
        let samples = plan.pressure.sample_layout();
        let n = source.layout().half_len();
        let m = diagnostic.layout().half_len();
        let points = samples.real_len();
        Ok(Self {
            products: ConservativeWorkspace::new(source, plan.bounds.storage_bytes)?,
            derivatives: DerivativeWorkspace::new(diagnostic, samples, plan.bounds.storage_bytes)?,
            provider: plan
                .pressure
                .force_settings()
                .build(diagnostic, plan.pressure.provider_limits().storage_bytes)?,
            velocity: field(n)?,
            force: field(m)?,
            conservative: field(m)?,
            pressure: complex(m)?,
            actual: real(points)?,
            errors: real(points)?,
            references: real(points)?,
            analytical: filled(points, zero_reference())?,
            charged: PressureReferenceWork::default(),
            next: 0,
            failed: false,
            plan,
        })
    }
    /// Remaining complete attempts; numerical failure terminates publication.
    pub fn remaining(&self) -> usize {
        self.plan.bounds.work.attempts - self.charged.attempts
    }
    /// Conservatively charged work including failed attempts.
    pub fn charged_work(&self) -> PressureReferenceWork {
        self.charged
    }
    /// Measure all branches before publishing one immutable report.
    pub fn measure(
        &mut self,
        family: &V2Family<'_>,
    ) -> Result<PressureReferenceSample<'a>, FamilyError> {
        self.charge()?;
        let clock = self
            .plan
            .pressure
            .family_plan()
            .require_sample(family, self.next)?;
        let gauge = self.plan.gauges[self.next];
        match self.compute(family, clock, gauge) {
            Ok(sample) => {
                self.next += 1;
                Ok(sample)
            }
            Err(error) => {
                self.failed = true;
                Err(error.into())
            }
        }
    }
    fn charge(&mut self) -> Result<(), FamilyError> {
        if self.failed {
            return Err(FamilyError::Terminated);
        }
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.charged.attempts += 1;
        self.charged.reference_evaluations += self.plan.per_attempt.reference_evaluations;
        self.charged.root_iterations += self.plan.per_attempt.root_iterations;
        self.charged.provider_work_units += self.plan.per_attempt.provider_work_units;
        self.charged.scalar_transforms += self.plan.per_attempt.scalar_transforms;
        self.charged.weighted_visits += self.plan.per_attempt.weighted_visits;
        Ok(())
    }
    fn compute(
        &mut self,
        family: &V2Family<'_>,
        clock: TickClock,
        gauge: ImportedGauge<'a>,
    ) -> Result<PressureReferenceSample<'a>, SolverError> {
        self.evaluate_force(clock)?;
        self.evaluate_reference(clock, gauge.mean())?;
        let branches = [0, 1, 2, 3, 4, 5]
            .map(|index| self.branch(family.branches[index].state(), index))
            .map(|x| x);
        let [a, b, c, d, e, f] = branches;
        Ok(PressureReferenceSample {
            clock,
            family_identity: self.plan.pressure.family_plan().identity(),
            sample_layout: self.plan.sample_layout(),
            force_layout: self.plan.force_layout(),
            gauge,
            branches: [a?, b?, c?, d?, e?, f?],
            charged: self.charged,
        })
    }
    fn evaluate_force(&mut self, clock: TickClock) -> Result<(), SolverError> {
        let limits = self.plan.pressure.provider_limits();
        let work =
            self.provider
                .evaluate(clock, limits, self.force.each_mut().map(Vec::as_mut_slice))?;
        if work.work_units > limits.work_units || work.scalar_transforms > limits.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn evaluate_reference(&mut self, clock: TickClock, mean: f64) -> Result<(), SolverError> {
        let time = BenchmarkTime::new(clock).map_err(|_| SolverError::InvalidPayload)?;
        let [nx, ny, nz] = self.plan.sample_layout().dimensions();
        for (index, value) in self.analytical.iter_mut().enumerate() {
            let point = [
                (index / (ny * nz)) as f64 / nx as f64,
                ((index / nz) % ny) as f64 / ny as f64,
                (index % nz) as f64 / nz as f64,
            ];
            *value = reference::evaluate(point, time).map_err(|_| SolverError::InvalidPayload)?;
            value.pressure_raw -= mean;
        }
        Ok(())
    }
    fn branch(
        &mut self,
        state: &SpectralState,
        branch: usize,
    ) -> Result<BranchPressureTracking, SolverError> {
        construct_pressure(
            PressureScratch {
                products: &mut self.products,
                velocity: &mut self.velocity,
                force: &self.force,
                conservative: &mut self.conservative,
            },
            self.plan.pressure.source_domain(),
            state.plan().domain(),
            [
                state.component(0)?,
                state.component(1)?,
                state.component(2)?,
            ],
            &mut self.pressure,
        )?;
        Ok(BranchPressureTracking {
            branch,
            pressure: self.quantity(false)?,
            pressure_gradient: self.quantity(true)?,
        })
    }
    fn quantity(&mut self, gradient: bool) -> Result<LocalError, SolverError> {
        self.errors.fill(0.0);
        self.references.fill(0.0);
        let count = if gradient { 3 } else { 1 };
        for component in 0..count {
            let mut orders = [0; 3];
            if gradient {
                orders[component] = 1;
            }
            self.actual.copy_from_slice(
                self.derivatives
                    .sample(&self.pressure, Derivative::new(orders)?)?
                    .values,
            );
            for index in 0..self.actual.len() {
                let expected = if gradient {
                    self.analytical[index].pressure_gradient[component]
                } else {
                    self.analytical[index].pressure_raw
                };
                self.errors[index] = self.errors[index].hypot(self.actual[index] - expected);
                self.references[index] = self.references[index].hypot(expected);
            }
        }
        if gradient {
            reduce::<3>(
                &self.errors,
                &self.references,
                self.plan.pressure.relative_floors()[1],
            )
        } else {
            reduce::<1>(
                &self.errors,
                &self.references,
                self.plan.pressure.relative_floors()[0],
            )
        }
    }
}
fn reduce<const C: usize>(
    errors: &[f64],
    references: &[f64],
    floor: f64,
) -> Result<LocalError, SolverError> {
    let mut out = TensorErrors::<C>::new(errors.len(), floor)?;
    for (&e, &r) in errors.iter().zip(references) {
        out.push_magnitudes(e, r)?;
    }
    match out.finish()? {
        SampledError::Measured(v) => Ok(v),
        SampledError::NoSamples => Err(SolverError::InvalidPayload),
    }
}
fn field(n: usize) -> Result<Field, SolverError> {
    Ok([complex(n)?, complex(n)?, complex(n)?])
}
fn complex(n: usize) -> Result<Vec<Complex64>, SolverError> {
    filled(n, Complex64::new(0.0, 0.0))
}
fn real(n: usize) -> Result<Vec<f64>, SolverError> {
    filled(n, 0.0)
}
fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    v.resize(n, value);
    Ok(v)
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
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
