//! Independent explicit first-derivative formulas for v2 reference velocity and raw pressure.
use crate::{root, time::BenchmarkTime, BenchmarkError};

/// Scalar analytical field sample; this type is never accepted by the timestep state constructor.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceFields {
    /// Three Cartesian velocity components.
    pub velocity: [f64; 3],
    /// Scalar root work and arithmetic evidence; absent in exact flat regions.
    pub root: Option<root::RootReport>,
    /// Kinematic pressure before subtracting its periodic spatial mean.
    pub pressure_raw: f64,
}

/// Evaluate the periodic scalar reference through explicit derivatives, independently of jets.
pub fn evaluate(point: [f64; 3], time: BenchmarkTime) -> Result<ReferenceFields, BenchmarkError> {
    let [x, y, z] = periodic(point)?;
    let radius2 = x * x + y * y + z * z;
    if radius2 >= 441.0 / 2500.0 || time.elapsed() == 0.0 {
        return Ok(ReferenceFields {
            velocity: [0.0; 3],
            pressure_raw: 0.0,
            root: None,
        });
    }
    let solution = root::solve(z, time.remaining(), 128)?;
    let q = solution.value;
    let (shape, shape_derivative) = smooth_step((441.0 / 2500.0 - radius2) / (54.0 / 625.0))?;
    let ramp = smooth_step(512.0 * time.elapsed())?.0;
    let cutoff = shape * ramp;
    let cutoff_r2 = -shape_derivative * ramp / (54.0 / 625.0);
    let eta = z * q.powf(-3.0 / 8.0);
    let radial = (x * x + y * y) / (2.0 * q);
    let gaussian = (-radial).exp();
    let amplitude = q.powf(-5.0 / 8.0) * gaussian / 2.0;
    let g = amplitude * (eta + 1.0 / 32.0);
    let swirl = q.powf(-9.0 / 8.0) * gaussian / 4.0;
    let qz = 2.0 * z * q.sqrt().sqrt() / (1.0 - eta * eta / 4.0);
    let etaz = q.powf(-3.0 / 8.0) - (3.0 / 8.0) * eta * qz / q;
    let gz = amplitude * (etaz + (eta + 1.0 / 32.0) * (radial - 5.0 / 8.0) * qz / q);
    let hz = 2.0 * z * cutoff_r2 * g + cutoff * gz;
    let radial_h = 2.0 * cutoff_r2 * g - cutoff * g / q;
    let velocity = [
        -x * hz - y * cutoff * swirl,
        -y * hz + x * cutoff * swirl,
        2.0 * cutoff * g + (x * x + y * y) * radial_h,
    ];
    let pressure_raw = -cutoff * cutoff * q.powf(-5.0 / 4.0) * (-2.0 * radial).exp() / 32.0;
    // The admitted tau >= 2^-134 and compact radius bound keep every scalar
    // intermediate below 2^400 in magnitude. Finite inputs cannot overflow here.
    Ok(ReferenceFields {
        velocity,
        pressure_raw,
        root: Some(solution),
    })
}

pub(crate) fn periodic(point: [f64; 3]) -> Result<[f64; 3], BenchmarkError> {
    if point.iter().any(|v| !v.is_finite()) {
        return Err(BenchmarkError::InvalidInput);
    }
    Ok(point.map(|x| {
        // Preserve already-centered coordinates, especially tiny negative values.
        if (-0.5..0.5).contains(&x) {
            return x;
        }
        let phase = x.rem_euclid(1.0);
        if phase >= 0.5 {
            phase - 1.0
        } else {
            phase
        }
    }))
}

/// Smooth step and its first derivative, including rounded tails without 0*infinity.
pub fn smooth_step(s: f64) -> Result<(f64, f64), BenchmarkError> {
    if !s.is_finite() {
        return Err(BenchmarkError::InvalidInput);
    }
    if s <= 0.0 {
        return Ok((0.0, 0.0));
    }
    if s >= 1.0 {
        return Ok((1.0, 0.0));
    }
    let ratio = 1.0 / (1.0 - s) - 1.0 / s;
    let exponential = (-ratio.abs()).exp();
    let value = if ratio <= 0.0 {
        exponential / (1.0 + exponential)
    } else {
        1.0 / (1.0 + exponential)
    };
    let a = -2.0 * s.ln();
    let b = -2.0 * (1.0 - s).ln();
    let maximum = a.max(b);
    let log_sum = maximum + ((a - maximum).exp() + (b - maximum).exp()).ln();
    let log_derivative = -ratio.abs() - 2.0 * exponential.ln_1p() + log_sum;
    Ok((value, log_derivative.exp()))
}

#[cfg(test)]
mod tests {
    use super::periodic;
    #[test]
    fn centering_preserves_small_values_and_has_one_boundary_convention() {
        assert_eq!(
            periodic([-1e-30, 0.25, -0.25]).unwrap(),
            [-1e-30, 0.25, -0.25]
        );
        assert_eq!(periodic([-0.5, 0.5, 1.5]).unwrap(), [-0.5; 3]);
        assert_eq!(
            periodic([1.25, -1.25, f64::MAX]).unwrap(),
            [0.25, -0.25, 0.0]
        );
    }
}
