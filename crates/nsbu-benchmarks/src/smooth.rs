//! Smooth nontrivial manufactured flow with exact Fourier forcing coefficients.
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::{
        forcing::{ForceLimits, ForceWork, PrescribedForce},
        time::binary_duration,
    },
    Complex64, SolverError,
};

/// Cyclic sine flow: u=(sin(13t)sin(2πy),sin(17t)sin(2πz),sin(19t)sin(2πx)), raw pressure zero.
/// The prescribed force includes all temporal, viscous and quadratic advection terms.
#[derive(Debug)]
pub struct CyclicSine {
    domain: Domain,
    limits: ForceLimits,
}

impl CyclicSine {
    /// Require the unit periodic cube; viscosity is the domain's positive constant.
    pub fn new(domain: Domain) -> Result<Self, SolverError> {
        if domain.lengths() != [1.0; 3] {
            return Err(SolverError::InvalidDomain);
        }
        let work_units = domain
            .layout()
            .half_len()
            .checked_mul(3)
            .ok_or(SolverError::SizeOverflow)?;
        Ok(Self {
            domain,
            limits: ForceLimits {
                storage_bytes: std::mem::size_of::<Self>(),
                work_units,
                scalar_transforms: 0,
                remaining_divisor: 1,
            },
        })
    }
}

impl PrescribedForce for CyclicSine {
    fn limits(&self) -> Option<ForceLimits> {
        Some(self.limits)
    }
    fn evaluate(
        &mut self,
        clock: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if limit != self.limits {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        let layout = self.domain.layout();
        if output
            .iter()
            .any(|values| values.len() != layout.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let amplitudes = amplitude(clock)?;
        let k = std::f64::consts::TAU;
        for (axis, values) in output.into_iter().enumerate() {
            let (g, derivative) = amplitudes[axis];
            let linear = derivative + self.domain.viscosity() * k * k * g;
            let quadratic = k * g * amplitudes[(axis + 1) % 3].0;
            if !linear.is_finite() {
                return Err(SolverError::ArithmeticResolutionLimited);
            }
            for (index, value) in values.iter_mut().enumerate() {
                let mode = layout.mode(layout.position(index)?)?;
                *value = coefficient(mode, axis, linear, quadratic);
            }
        }
        Ok(ForceWork {
            work_units: limit.work_units,
            scalar_transforms: 0,
        })
    }
}

/// Three independent temporal amplitudes and their physical-time derivatives.
pub fn amplitude(clock: TickClock) -> Result<[(f64, f64); 3], SolverError> {
    let time = if clock.elapsed() == 0 {
        0.0
    } else {
        binary_duration(clock.elapsed(), clock.exponent())?
    };
    let mut values = [(0.0, 0.0); 3];
    for (value, frequency) in values.iter_mut().zip([13.0, 17.0, 19.0]) {
        let phase = frequency * time;
        if !phase.is_finite() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        *value = (phase.sin(), frequency * phase.cos());
    }
    Ok(values)
}

fn coefficient(mode: [isize; 3], axis: usize, linear: f64, quadratic: f64) -> Complex64 {
    let sine = (axis + 1) % 3;
    let advector = (axis + 2) % 3;
    let imaginary = if mode[axis] != 0 || mode[sine].unsigned_abs() != 1 {
        0.0
    } else if mode[advector] == 0 {
        orientation(linear / 2.0, mode[sine])
    } else if mode[advector].unsigned_abs() == 1 {
        orientation(quadratic / 4.0, mode[advector])
    } else {
        0.0
    };
    Complex64::new(0.0, imaginary)
}

// Only the phase orientation matters after the unit-frequency admission above.
fn orientation(amplitude: f64, mode: isize) -> f64 {
    if mode.is_negative() {
        amplitude
    } else {
        -amplitude
    }
}
