//! Bounded sampled vector errors; missing samples are never represented as zero error.
use super::squares::{finite, Squares};
use crate::{Complex64, SolverError};

/// Measurements on the supplied sample set, not rigorous local suprema or volume enclosures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalError {
    /// Number of actual samples in this measurement.
    pub samples: usize,
    /// Root mean squared Euclidean error on this sample set.
    pub rms_error: f64,
    /// Largest sampled Euclidean error.
    pub peak_error: f64,
    /// Largest error divided by max(pointwise reference magnitude, declared floor).
    pub peak_relative_error: f64,
    /// Largest sampled reference magnitude, retained for floor/scale review.
    pub reference_peak: f64,
    /// Explicit positive denominator floor in field units.
    pub relative_floor: f64,
}

/// Sampling absence is distinct from geometric region emptiness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampledError {
    /// No observations were supplied. This cannot pass an error budget as zero.
    NoSamples,
    /// Finite measured errors with an explicit positive relative floor.
    Measured(LocalError),
}

/// Fixed-storage transactional statistics; cloning copies measurements, never an integrated state.
#[derive(Debug, Clone, Copy)]
pub struct ErrorAccumulator {
    maximum: usize,
    count: usize,
    squares: Squares,
    peak: f64,
    relative_peak: f64,
    reference_peak: f64,
    floor: f64,
}
impl ErrorAccumulator {
    /// Admit finite sample capacity and a positive declared relative denominator floor.
    /// Experiments normally derive the floor from a separately measured reference peak,
    /// retaining an absolute positive floor for an exactly zero reference.
    pub fn new(maximum_samples: usize, relative_floor: f64) -> Result<Self, SolverError> {
        if maximum_samples == 0 || !relative_floor.is_finite() || relative_floor <= 0.0 {
            return Err(SolverError::InvalidPayload);
        }
        Ok(Self {
            maximum: maximum_samples,
            count: 0,
            squares: Squares::default(),
            peak: 0.0,
            relative_peak: 0.0,
            reference_peak: 0.0,
            floor: relative_floor,
        })
    }

    /// Add one physical vector comparison. Every failure preserves the previous statistics.
    pub fn push(&mut self, actual: [f64; 3], reference: [f64; 3]) -> Result<(), SolverError> {
        let mut pending = *self;
        pending.accumulate(actual, reference)?;
        *self = pending;
        Ok(())
    }

    fn accumulate(&mut self, actual: [f64; 3], reference: [f64; 3]) -> Result<(), SolverError> {
        if self.count == self.maximum {
            return Err(SolverError::ResourceLimit);
        }
        if actual
            .into_iter()
            .chain(reference)
            .any(|value| !value.is_finite())
        {
            return Err(SolverError::InvalidSpectrum);
        }
        let difference = std::array::from_fn::<_, 3, _>(|axis| actual[axis] - reference[axis]);
        let error = finite(difference[0].hypot(difference[1]).hypot(difference[2]))?;
        let scale = finite(reference[0].hypot(reference[1]).hypot(reference[2]))?;
        let relative = finite(error / scale.max(self.floor))?;
        for value in difference {
            self.squares.complex(Complex64::new(value, 0.0), 1.0)?;
        }
        self.count += 1;
        self.peak = self.peak.max(error);
        self.relative_peak = self.relative_peak.max(relative);
        self.reference_peak = self.reference_peak.max(scale);
        Ok(())
    }

    /// Preserve the explicit no-sample state. No inferred geometric coverage is returned.
    pub fn finish(self) -> Result<SampledError, SolverError> {
        if self.count == 0 {
            return Ok(SampledError::NoSamples);
        }
        Ok(SampledError::Measured(LocalError {
            samples: self.count,
            rms_error: self.squares.rms(self.count)?,
            peak_error: self.peak,
            peak_relative_error: self.relative_peak,
            reference_peak: self.reference_peak,
            relative_floor: self.floor,
        }))
    }
}
