//! Volume-average Fourier norms with derivative-sensitive channels.
use super::squares::{finite, Squares};
use crate::{Complex64, SolverError};

/// Norms of a Fourier vector field; all quantities use volume averages, not integrals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Norms {
    /// Square root of the volume-average squared velocity magnitude.
    pub l2: f64,
    /// Fourier H1 norm including the velocity and all first spatial derivatives.
    pub h1: f64,
    /// Volume-average L2 norm of curl.
    pub vorticity_l2: f64,
    /// Volume-average L2 norm of divergence.
    pub divergence_l2: f64,
}
impl Norms {
    pub(crate) fn orthogonal_sum(self, other: Self) -> Result<Self, SolverError> {
        Ok(Self {
            l2: finite(self.l2.hypot(other.l2))?,
            h1: finite(self.h1.hypot(other.h1))?,
            vorticity_l2: finite(self.vorticity_l2.hypot(other.vorticity_l2))?,
            divergence_l2: finite(self.divergence_l2.hypot(other.divergence_l2))?,
        })
    }
}

#[derive(Default)]
pub(crate) struct NormSums {
    l2: Squares,
    h1: Squares,
    curl: Squares,
    divergence: Squares,
}

/// Signed inner-product channels corresponding exactly to [`Norms`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedNormChannels {
    /// Signed L2 squared-norm contribution.
    pub l2: f64,
    /// Signed H1 squared-norm contribution.
    pub h1: f64,
    /// Signed curl squared-norm contribution.
    pub vorticity_l2: f64,
    /// Signed divergence squared-norm contribution.
    pub divergence_l2: f64,
}

#[derive(Default)]
pub(crate) struct CrossSums {
    l2: SignedSum,
    h1: SignedSum,
    curl: SignedSum,
    divergence: SignedSum,
}

impl CrossSums {
    /// Accumulate twice the real Hilbert inner product, including Parseval weight.
    pub(crate) fn push(
        &mut self,
        k: [f64; 3],
        a: [Complex64; 3],
        b: [Complex64; 3],
        weight: f64,
    ) -> Result<(), SolverError> {
        let dot = |x: Complex64, y: Complex64| x.re * y.re + x.im * y.im;
        let l2 = a.into_iter().zip(b).map(|(x, y)| dot(x, y)).sum::<f64>();
        let k2 = k.into_iter().map(|value| value * value).sum::<f64>();
        let curl_a = [
            k[1] * a[2] - k[2] * a[1],
            k[2] * a[0] - k[0] * a[2],
            k[0] * a[1] - k[1] * a[0],
        ];
        let curl_b = [
            k[1] * b[2] - k[2] * b[1],
            k[2] * b[0] - k[0] * b[2],
            k[0] * b[1] - k[1] * b[0],
        ];
        let curl = curl_a
            .into_iter()
            .zip(curl_b)
            .map(|(x, y)| dot(x, y))
            .sum::<f64>();
        let divergence_a = k[0] * a[0] + k[1] * a[1] + k[2] * a[2];
        let divergence_b = k[0] * b[0] + k[1] * b[1] + k[2] * b[2];
        let factor = 2.0 * weight;
        self.l2.push(factor * l2)?;
        self.h1.push(factor * (1.0 + k2) * l2)?;
        self.curl.push(factor * curl)?;
        self.divergence
            .push(factor * dot(divergence_a, divergence_b))
    }

    pub(crate) fn finish(self) -> Result<SignedNormChannels, SolverError> {
        Ok(SignedNormChannels {
            l2: self.l2.finish()?,
            h1: self.h1.finish()?,
            vorticity_l2: self.curl.finish()?,
            divergence_l2: self.divergence.finish()?,
        })
    }
}

#[derive(Default)]
struct SignedSum {
    sum: f64,
    correction: f64,
}

impl SignedSum {
    fn push(&mut self, value: f64) -> Result<(), SolverError> {
        finite(value)?;
        let next = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.correction += (self.sum - next) + value;
        } else {
            self.correction += (value - next) + self.sum;
        }
        self.sum = next;
        finite(self.sum)?;
        finite(self.correction)?;
        Ok(())
    }

    fn finish(self) -> Result<f64, SolverError> {
        finite(self.sum + self.correction)
    }
}
impl NormSums {
    pub(crate) fn push(
        &mut self,
        k: [f64; 3],
        u: [Complex64; 3],
        weight: f64,
    ) -> Result<(), SolverError> {
        for value in u {
            self.l2.complex(value, weight)?;
            self.h1.complex(value, weight)?;
            for frequency in k {
                self.h1.complex(value * frequency, weight)?;
            }
        }
        // Multiplication by i changes neither curl nor divergence norms.
        for (next, last) in [(1, 2), (2, 0), (0, 1)] {
            self.curl
                .complex(k[next] * u[last] - k[last] * u[next], weight)?;
        }
        self.divergence
            .complex(k[0] * u[0] + k[1] * u[1] + k[2] * u[2], weight)
    }
    pub(crate) fn finish(self) -> Result<Norms, SolverError> {
        Ok(Norms {
            l2: self.l2.norm()?,
            h1: self.h1.norm()?,
            vorticity_l2: self.curl.norm()?,
            divergence_l2: self.divergence.norm()?,
        })
    }
}
