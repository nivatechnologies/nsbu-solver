//! Independent full-double-band PDE defects at genuine off-stage accepted-history probes.
use super::FamilyError;
use crate::{
    smooth::CyclicSine,
    smooth_run::{Origin, ReconstructedRun},
};
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, norms::Norms, residual::ResidualPlan},
    domain::{Domain, TickClock},
    integrators::forcing::{ForceLimits, PrescribedForce},
    verification::reconstruction::OffStageProbe,
    Complex64, SolverError,
};
mod reservation;
pub use reservation::{ResidualBounds, ResidualWork};
type Field = [Vec<Complex64>; 3];

/// A sampled residual with its actual accepted clocks and origin, never a continuous-time bound.
#[derive(Debug, Clone, Copy)]
pub struct ResidualSample {
    geometry: OffStageProbe,
    origin: Origin,
    domain: Domain,
    norms: Norms,
}
impl ResidualSample {
    /// Complete-double-band L2, H1 and vorticity defect norms at this exact probe.
    pub fn norms(self) -> Norms {
        self.norms
    }
    /// Accepted macro-endpoint geometry used for independent Hermite reconstruction.
    pub fn geometry(self) -> OffStageProbe {
        self.geometry
    }
    /// Origin restrictions survive observation; external input is never authenticated by a norm.
    pub fn origin(self) -> Origin {
        self.origin
    }
    /// Retained source geometry; residual assembly uses twice each axis.
    pub fn domain(self) -> Domain {
        self.domain
    }
}

/// Dedicated diagnostic buffers and prescribed provider; integration scratch is never borrowed.
pub struct ResidualWorkspace {
    source: Domain,
    bounds: ResidualBounds,
    force_limit: ForceLimits,
    force: CyclicSine,
    products: ConservativeWorkspace,
    value: Field,
    derivative: Field,
    forcing: Field,
    conservative: Field,
    residual: Field,
    pressure: Vec<Complex64>,
    work: ResidualWork,
}
impl ResidualWorkspace {
    /// Preflight the full diagnostic allocation and finite worst-case work before constructing it.
    pub fn new(source: Domain, maximum_probes: usize, cap: usize) -> Result<Self, SolverError> {
        let bounds = Self::reservation(source, maximum_probes)?;
        if bounds.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force = CyclicSine::new(diagnostic)?;
        let force_limit = force.limits().ok_or(SolverError::UnknownProviderCost)?;
        let n = source.layout().half_len();
        let m = diagnostic.layout().half_len();
        Ok(Self {
            source,
            bounds,
            force_limit,
            force,
            products: ConservativeWorkspace::new(
                source,
                ConservativeWorkspace::reservation(source)?,
            )?,
            value: field(n)?,
            derivative: field(n)?,
            forcing: field(m)?,
            conservative: field(m)?,
            residual: field(m)?,
            pressure: filled(m)?,
            work: ResidualWork::default(),
        })
    }
    /// Immutable storage/work admission including all interpolation and double-grid fields.
    pub fn bounds(&self) -> ResidualBounds {
        self.bounds
    }
    /// Conservatively charged work. Failed probes keep the complete reserved attempt charge.
    pub fn consumption(&self) -> ResidualWork {
        self.work
    }
    /// Remaining probe attempts; neither failure nor a different supplied run replenishes them.
    pub fn remaining(&self) -> usize {
        self.bounds.maximum_probes - self.work.probes
    }
    /// Reconstruct actual accepted data and form v_t + P div(v tensor v) - nu Delta v - P f.
    ///
    /// The force is evaluated on the complete doubled grid at the exact probe. No stage RHS or
    /// analytical reference is used to assemble the defect. Even a small result is a sampled
    /// diagnostic and requires separate reconstruction, space, force and arithmetic refinements.
    pub fn measure(
        &mut self,
        run: &ReconstructedRun,
        probe: TickClock,
    ) -> Result<ResidualSample, FamilyError> {
        self.charge()?;
        if run.state().plan().domain() != self.source {
            return Err(SolverError::InvalidDomain.into());
        }
        let nodes = run
            .observer()
            .last_accepted_clocks()
            .ok_or(nsbu_solver::verification::VerificationError::MissingReconstruction)?;
        let geometry = OffStageProbe::new(nodes, probe)?;
        for axis in 0..3 {
            run.observer().reconstruct(
                probe,
                axis,
                &mut self.value[axis],
                &mut self.derivative[axis],
            )?;
        }
        self.evaluate_force(probe)?;
        self.products.evaluate(
            self.value.each_ref().map(Vec::as_slice),
            self.forcing.each_ref().map(Vec::as_slice),
            self.conservative.each_mut().map(Vec::as_mut_slice),
            &mut self.pressure,
        )?;
        let norms = ResidualPlan::new(self.source)?.evaluate(
            self.value.each_ref().map(Vec::as_slice),
            self.derivative.each_ref().map(Vec::as_slice),
            self.conservative.each_ref().map(Vec::as_slice),
            self.residual.each_mut().map(Vec::as_mut_slice),
        )?;
        Ok(ResidualSample {
            geometry,
            origin: run.origin(),
            domain: self.source,
            norms,
        })
    }
    fn evaluate_force(&mut self, probe: TickClock) -> Result<(), SolverError> {
        let spent = self.force.evaluate(
            probe,
            self.force_limit,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        if spent.work_units > self.force_limit.work_units
            || spent.scalar_transforms > self.force_limit.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.remaining() == 0 {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        // All products were checked against maximum_probes before allocation. No partial ledger
        // update can overflow, and no fallible work follows until the full charge is retained.
        self.work.probes += 1;
        self.work.provider_work_units +=
            self.bounds.work.provider_work_units / self.bounds.maximum_probes;
        self.work.scalar_transforms +=
            self.bounds.work.scalar_transforms / self.bounds.maximum_probes;
        self.work.coefficient_work_units +=
            self.bounds.work.coefficient_work_units / self.bounds.maximum_probes;
        Ok(())
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
