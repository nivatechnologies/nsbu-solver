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
