//! Pure modal incompressibility, curl and modified-pressure operators.
use crate::{domain::Domain, Complex64, SolverError};

/// Three Cartesian complex coefficients at one wavevector.
pub type Vector = [Complex64; 3];

/// Convert a strict-band integer mode to finite physical wave numbers.
pub fn wavevector(domain: Domain, mode: [isize; 3]) -> Result<[f64; 3], SolverError> {
    domain.layout().locate(mode)?;
    let lengths = domain.lengths();
    let k = std::array::from_fn(|i| std::f64::consts::TAU * mode[i] as f64 / lengths[i]);
    if k.iter().any(|v| !v.is_finite()) {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(k)
}

/// Helmholtz projection with scaled wave numbers, preserving the exact mean.
pub fn project(k: [f64; 3], vector: Vector) -> Result<Vector, SolverError> {
    let (scale, direction, dot) = longitudinal(k, vector)?;
    if scale == 0.0 {
        return Ok(vector);
    }
    let result = std::array::from_fn(|i| vector[i] - direction[i] * dot);
    validate(k, result)?;
    Ok(result)
}

/// Fourier curl, including negative modes and the zero mean derivative.
pub fn curl(k: [f64; 3], vector: Vector) -> Result<Vector, SolverError> {
    validate(k, vector)?;
    let result = [
        Complex64::i() * (k[1] * vector[2] - k[2] * vector[1]),
        Complex64::i() * (k[2] * vector[0] - k[0] * vector[2]),
        Complex64::i() * (k[0] * vector[1] - k[1] * vector[0]),
    ];
    validate(k, result)?;
    Ok(result)
}

/// Modified pressure pi = p + |u|²/2, with zero spatial mean.
/// The input is the unprojected rotational term plus prescribed force.
pub fn modified_pressure(k: [f64; 3], acceleration: Vector) -> Result<Complex64, SolverError> {
    let (scale, _, dot) = longitudinal(k, acceleration)?;
    if scale == 0.0 {
        return Ok(Complex64::new(0.0, 0.0));
    }
    let pressure = -Complex64::i() * dot / scale;
    validate(k, [pressure; 3])?;
    Ok(pressure)
}

fn longitudinal(k: [f64; 3], vector: Vector) -> Result<(f64, [f64; 3], Complex64), SolverError> {
    validate(k, vector)?;
    let scale = k
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return Ok((0.0, [0.0; 3], Complex64::new(0.0, 0.0)));
    }
    let direction = k.map(|value| value / scale);
    let squared = direction.iter().map(|value| value * value).sum::<f64>();
    let dot = direction
        .iter()
        .zip(vector)
        .map(|(a, b)| a * b)
        .sum::<Complex64>()
        / squared;
    Ok((scale, direction, dot))
}

fn validate(k: [f64; 3], vector: Vector) -> Result<(), SolverError> {
    if k.iter().any(|value| !value.is_finite())
        || vector
            .iter()
            .any(|value| !value.re.is_finite() || !value.im.is_finite())
    {
        return Err(SolverError::InvalidSpectrum);
    }
    Ok(())
}
