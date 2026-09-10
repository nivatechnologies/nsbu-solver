//! Quintic reconstruction weights on exact start/midpoint/endpoint clocks.
use super::squares::finite;
use crate::{domain::TickClock, integrators::time::binary_duration, Complex64, SolverError};

const BASIS: [[f64; 6]; 6] = [
    [1.0, 0.0, -23.0, 66.0, -68.0, 24.0],
    [0.0, 0.0, 16.0, -32.0, 16.0, 0.0],
    [0.0, 0.0, 7.0, -34.0, 52.0, -24.0],
    [0.0, 1.0, -6.0, 13.0, -12.0, 4.0],
    [0.0, 0.0, -8.0, 32.0, -40.0, 16.0],
    [0.0, 0.0, -1.0, 5.0, -8.0, 4.0],
];

/// Floating reconstruction coefficients, not rigorous error enclosures.
/// Inputs are three values followed by their three physical-time derivatives.
#[derive(Debug, Clone, Copy)]
pub struct HermiteWeights {
    /// Coefficients for the reconstructed field.
    pub value: [f64; 6],
    /// Coefficients for its physical-time derivative.
    pub derivative: [f64; 6],
}

impl HermiteWeights {
    /// Require matching clocks and an exact, equally spaced middle node.
    /// This does not establish that caller-supplied data came from accepted states.
    pub fn at(nodes: [TickClock; 3], probe: TickClock) -> Result<Self, SolverError> {
        let (position, duration) = coordinates(nodes, probe)?;
        Self::weights(position, duration)
    }

    /// Reconstruct one borrowed spectral component and its time derivative into caller scratch.
    /// Samples are ordered as three node values followed by three node derivatives.
    /// No allocation or access to mutable integrated state is performed.
    pub fn apply(
        self,
        samples: [&[Complex64]; 6],
        value: &mut [Complex64],
        derivative: &mut [Complex64],
    ) -> Result<(), SolverError> {
        if samples.iter().any(|sample| sample.len() != value.len())
            || derivative.len() != value.len()
        {
            return Err(SolverError::InvalidPayload);
        }
        for weight in self.value.into_iter().chain(self.derivative) {
            finite(weight)?;
        }
        for (index, (value, derivative)) in value.iter_mut().zip(derivative).enumerate() {
            let mut v = Complex64::new(0.0, 0.0);
            let mut d = v;
            for (node, sample) in samples.into_iter().enumerate() {
                finite(sample[index].re)?;
                finite(sample[index].im)?;
                v += self.value[node] * sample[index];
                d += self.derivative[node] * sample[index];
            }
            finite(v.re)?;
            finite(v.im)?;
            finite(d.re)?;
            finite(d.im)?;
            *value = v;
            *derivative = d;
        }
        Ok(())
    }

    fn weights(position: f64, duration: f64) -> Result<Self, SolverError> {
        let mut result = Self {
            value: [0.0; 6],
            derivative: [0.0; 6],
        };
        for (index, coefficients) in BASIS.into_iter().enumerate() {
            let mut value = coefficients[5];
            let mut derivative = 0.0;
            for coefficient in coefficients[..5].iter().rev() {
                derivative = derivative * position + value;
                value = value * position + coefficient;
            }
            let (value_scale, derivative_scale) = if index < 3 {
                (1.0, 1.0 / duration)
            } else {
                (duration, 1.0)
            };
            result.value[index] = finite(value * value_scale)?;
            result.derivative[index] = finite(derivative * derivative_scale)?;
        }
        Ok(result)
    }
}

/// Validate exact interval geometry without constructing interpolation coefficients.
pub(crate) fn coordinates(
    nodes: [TickClock; 3],
    probe: TickClock,
) -> Result<(f64, f64), SolverError> {
    let start = nodes[0];
    for clock in nodes.into_iter().chain([probe]) {
        if clock.exponent() != start.exponent() || clock.target() != start.target() {
            return Err(SolverError::InvalidClock);
        }
    }
    let first = nodes[1].elapsed().checked_sub(start.elapsed());
    let second = nodes[2].elapsed().checked_sub(nodes[1].elapsed());
    if first == Some(0) || first.is_none() || first != second {
        return Err(SolverError::InvalidClock);
    }
    if probe.elapsed() < start.elapsed() || probe.elapsed() > nodes[2].elapsed() {
        return Err(SolverError::InvalidClock);
    }
    let span = nodes[2].elapsed() - start.elapsed();
    let offset = probe.elapsed() - start.elapsed();
    let duration = binary_duration(span, start.exponent())?;
    let position = if offset == 0 {
        0.0
    } else {
        binary_duration(offset, start.exponent())? / duration
    };
    // Both durations were admitted exactly. Their positive ratio is at least
    // 2^-128, and exact binary64 offsets cannot collapse onto another node.
    Ok((position, duration))
}
