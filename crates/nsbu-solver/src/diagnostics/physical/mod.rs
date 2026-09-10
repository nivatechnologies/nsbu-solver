//! Full physical scalar/vector/tensor comparisons using bounded sequential component sampling.
mod quantity;
use super::{
    comparison::ComparisonPlan,
    derivatives::DerivativeWorkspace,
    local::{LocalError, SampledError, TensorErrors},
    squares::finite,
};
use crate::{
    domain::{Domain, Layout},
    storage::filled,
    SolverError,
};
pub use quantity::{PhysicalField, PhysicalQuantity};

/// Complete unaligned sample differences produced internally by the comparison workspace.
/// Mathematical problem, physical clock and trajectory provenance are bound by the experiment.
#[derive(Debug)]
pub struct PhysicalComparison<'a> {
    domains: [Domain; 2],
    samples: Layout,
    quantity: PhysicalQuantity,
    errors: &'a [f64],
    reference: &'a [f64],
    global: LocalError,
}
impl PhysicalComparison<'_> {
    /// Coarse/actual and fine/reference source domains, with every retained mode included.
    pub fn domains(&self) -> [Domain; 2] {
        self.domains
    }
    /// Exact physical quantity, including its complete ordered component inventory.
    pub fn quantity(&self) -> PhysicalQuantity {
        self.quantity
    }
    /// Diagnostic physical sample grid shared by both unchanged fields.
    pub fn sample_layout(&self) -> Layout {
        self.samples
    }
    /// Full-field difference magnitude at every sample, suitable for separate region aggregation.
    pub fn error_magnitudes(&self) -> &[f64] {
        self.errors
    }
    /// Complete reference field magnitude at the identical samples.
    pub fn reference_magnitudes(&self) -> &[f64] {
        self.reference
    }
    /// Global sampled RMS, absolute/relative peak and reference scale with explicit floor.
    pub fn global(&self) -> LocalError {
        self.global
    }
    /// Scalar transforms actually executed for this completed comparison.
    pub fn scalar_transforms(&self) -> usize {
        self.quantity.scalar_transforms()
    }
}

/// Two scalar samplers and four physical arrays, independent of tensor component count.
/// Input spectra remain borrowed and no integrated state or reference evaluator is owned.
#[derive(Debug)]
pub struct PhysicalComparisonWorkspace {
    domains: [Domain; 2],
    planned_bytes: usize,
    samples: Layout,
    left: DerivativeWorkspace,
    right: DerivativeWorkspace,
    left_component: Vec<f64>,
    right_component: Vec<f64>,
    error_magnitudes: Vec<f64>,
    reference_magnitudes: Vec<f64>,
}
impl PhysicalComparisonWorkspace {
    /// Complete owned element/header reservation; inputs and allocator overhead are separate.
    /// Physical lengths/viscosity must agree and the second source must retain the first band.
    pub fn reservation(left: Domain, right: Domain, samples: Layout) -> Result<usize, SolverError> {
        ComparisonPlan::new(left, right)?;
        let left_bytes = DerivativeWorkspace::reservation(left, samples)?;
        let right_bytes = DerivativeWorkspace::reservation(right, samples)?;
        samples
            .real_len()
            .checked_mul(4 * 8)
            .and_then(|n| n.checked_add(left_bytes))
            .and_then(|n| n.checked_add(right_bytes))
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(SolverError::SizeOverflow)
    }
    /// Admit the aggregate reservation before allocating either sampler or any physical array.
    pub fn new(
        left: Domain,
        right: Domain,
        samples: Layout,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let planned_bytes = Self::reservation(left, right, samples)?;
        if planned_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            domains: [left, right],
            planned_bytes,
            samples,
            left: DerivativeWorkspace::new(left, samples, cap)?,
            right: DerivativeWorkspace::new(right, samples, cap)?,
            left_component: filled(samples.real_len(), 0.0)?,
            right_component: filled(samples.real_len(), 0.0)?,
            error_magnitudes: filled(samples.real_len(), 0.0)?,
            reference_magnitudes: filled(samples.real_len(), 0.0)?,
        })
    }
    /// Compare complete fields; tensor entries are sampled sequentially in fixed order.
    /// The reference supplies the relative scale. It is never assigned to an evolving state.
    /// No output view is issued on failure; scratch can be reused and input bytes are unchanged.
    pub fn compare(
        &mut self,
        left: PhysicalField<'_>,
        right: PhysicalField<'_>,
        quantity: PhysicalQuantity,
        relative_floor: f64,
    ) -> Result<PhysicalComparison<'_>, SolverError> {
        self.compare_domains(self.domains, left, right, quantity, relative_floor)
    }

    /// Reuse the same preallocated samplers for another admitted source-grid pair.
    /// Physical lengths/viscosity stay fixed; the sample grid and complete reservation
    /// cannot grow. The returned report records these actual domains. `compare` continues
    /// to use the original constructor domains, including after a refused alternate call.
    pub fn compare_domains(
        &mut self,
        domains: [Domain; 2],
        left: PhysicalField<'_>,
        right: PhysicalField<'_>,
        quantity: PhysicalQuantity,
        relative_floor: f64,
    ) -> Result<PhysicalComparison<'_>, SolverError> {
        if domains[0].lengths() != self.domains[0].lengths()
            || domains[0].viscosity() != self.domains[0].viscosity()
        {
            return Err(SolverError::InvalidDomain);
        }
        if Self::reservation(domains[0], domains[1], self.samples)? > self.planned_bytes {
            return Err(SolverError::ResourceLimit);
        }
        quantity.admit(left)?;
        quantity.admit(right)?;
        TensorErrors::<1>::new(self.samples.real_len(), relative_floor)?;
        self.left.bind_source(domains[0]);
        self.right.bind_source(domains[1]);
        self.error_magnitudes.fill(0.0);
        self.reference_magnitudes.fill(0.0);
        for component in 0..quantity.components() {
            quantity.sample(component, left, &mut self.left, &mut self.left_component)?;
            quantity.sample(component, right, &mut self.right, &mut self.right_component)?;
            self.accumulate()?;
        }
        let global = self.summarize(quantity, relative_floor)?;
        Ok(PhysicalComparison {
            domains,
            samples: self.samples,
            quantity,
            errors: &self.error_magnitudes,
            reference: &self.reference_magnitudes,
            global,
        })
    }
    fn accumulate(&mut self) -> Result<(), SolverError> {
        for index in 0..self.samples.real_len() {
            let difference = finite(self.left_component[index] - self.right_component[index])?;
            self.error_magnitudes[index] = finite(self.error_magnitudes[index].hypot(difference))?;
            self.reference_magnitudes[index] =
                finite(self.reference_magnitudes[index].hypot(self.right_component[index]))?;
        }
        Ok(())
    }
    fn summarize(&self, quantity: PhysicalQuantity, floor: f64) -> Result<LocalError, SolverError> {
        match quantity {
            PhysicalQuantity::Scalar => self.reduce::<1>(floor),
            PhysicalQuantity::ScalarGradient
            | PhysicalQuantity::Vector
            | PhysicalQuantity::Vorticity => self.reduce::<3>(floor),
            PhysicalQuantity::Gradient => self.reduce::<9>(floor),
            PhysicalQuantity::Hessian => self.reduce::<27>(floor),
        }
    }
    fn reduce<const C: usize>(&self, floor: f64) -> Result<LocalError, SolverError> {
        let mut statistics = TensorErrors::<C>::new(self.samples.real_len(), floor)?;
        for (&error, &reference) in self.error_magnitudes.iter().zip(&self.reference_magnitudes) {
            statistics.push_magnitudes(error, reference)?;
        }
        match statistics.finish()? {
            SampledError::Measured(value) => Ok(value),
            SampledError::NoSamples => Err(SolverError::InvalidPayload),
        }
    }
}
