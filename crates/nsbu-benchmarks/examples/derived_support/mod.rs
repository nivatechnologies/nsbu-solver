//! Independent diagnostic scratch for exact-bit export after a bounded actual trajectory.
mod output;
use crate::arithmetic_support::ExportError;
use nsbu_benchmarks::{smooth::CyclicSine, smooth_run::ReconstructedRun};
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, derivatives::DerivativeWorkspace},
    domain::Domain,
    integrators::forcing::{ForceLimits, PrescribedForce},
    spectral::modal,
    Complex64, SolverError,
};
pub struct Workspace {
    source: Domain,
    pub(super) scalar: DerivativeWorkspace,
    pub(super) pressure_scalar: DerivativeWorkspace,
    products: ConservativeWorkspace,
    pub(super) force: CyclicSine,
    limit: ForceLimits,
    pub(super) force_values: [Vec<Complex64>; 3],
    conservative: [Vec<Complex64>; 3],
    pub(super) pressure: Vec<Complex64>,
    pub(super) curl: [Vec<Complex64>; 3],
}
impl Workspace {
    pub fn reservation(source: Domain) -> Result<usize, SolverError> {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let sample = diagnostic.layout();
        let elements = source
            .layout()
            .half_len()
            .checked_mul(3)
            .and_then(|n| {
                sample
                    .half_len()
                    .checked_mul(7)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| n.checked_mul(16))
            .ok_or(SolverError::SizeOverflow)?;
        [
            elements,
            DerivativeWorkspace::reservation(source, sample)?,
            DerivativeWorkspace::reservation(diagnostic, sample)?,
            std::mem::size_of::<Self>(),
        ]
        .into_iter()
        .try_fold(ConservativeWorkspace::reservation(source)?, |n, m| {
            n.checked_add(m).ok_or(SolverError::SizeOverflow)
        })
    }
    pub fn new(source: Domain, cap: usize) -> Result<Self, SolverError> {
        if Self::reservation(source)? > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let sample = diagnostic.layout();
        let force = CyclicSine::new(diagnostic)?;
        let limit = force.limits().ok_or(SolverError::UnknownProviderCost)?;
        Ok(Self {
            source,
            scalar: DerivativeWorkspace::new(source, sample, cap)?,
            pressure_scalar: DerivativeWorkspace::new(diagnostic, sample, cap)?,
            products: ConservativeWorkspace::new(source, cap)?,
            force,
            limit,
            force_values: field(sample.half_len())?,
            conservative: field(sample.half_len())?,
            pressure: filled(sample.half_len())?,
            curl: field(source.layout().half_len())?,
        })
    }
    pub fn evaluate(&mut self, run: &ReconstructedRun) -> Result<(), SolverError> {
        let state = run.state();
        let source = state.plan().domain();
        if source != self.source {
            return Err(SolverError::InvalidDomain);
        }
        let work = self.force.evaluate(
            state.clock(),
            self.limit,
            self.force_values.each_mut().map(Vec::as_mut_slice),
        )?;
        if work.work_units > self.limit.work_units
            || work.scalar_transforms > self.limit.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        let values = [
            state.component(0)?,
            state.component(1)?,
            state.component(2)?,
        ];
        self.products.evaluate(
            values,
            self.force_values.each_ref().map(Vec::as_slice),
            self.conservative.each_mut().map(Vec::as_mut_slice),
            &mut self.pressure,
        )?;
        self.build_curl(source, values)
    }
    fn build_curl(&mut self, source: Domain, values: [&[Complex64]; 3]) -> Result<(), SolverError> {
        let layout = source.layout();
        for index in 0..layout.half_len() {
            let position = layout.position(index)?;
            let v = if layout.is_nyquist(position)? {
                [Complex64::new(0.0, 0.0); 3]
            } else {
                modal::curl(
                    modal::wavevector(source, layout.mode(position)?)?,
                    values.map(|v| v[index]),
                )?
            };
            for (field, value) in self.curl.iter_mut().zip(v) {
                field[index] = value;
            }
        }
        Ok(())
    }
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    v.resize(n, Complex64::new(0.0, 0.0));
    Ok(v)
}
fn field(n: usize) -> Result<[Vec<Complex64>; 3], SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
