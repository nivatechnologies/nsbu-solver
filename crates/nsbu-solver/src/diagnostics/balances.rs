//! Independent Parseval balance terms, all expressed as physical volume averages.
use super::{
    norms::{NormSums, Norms},
    squares::{finite, Squares},
    summation::Sum,
};
use crate::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64, SolverError,
};

/// Instantaneous measured quantities. No quadrature or continuous-time claim is implied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BalanceSample {
    /// Field norms, including the incompressibility diagnostic.
    pub norms: Norms,
    /// Half the volume average of squared velocity.
    pub energy: f64,
    /// Half the volume average of squared vorticity.
    pub enstrophy: f64,
    /// nu times the volume average of squared vorticity.
    pub energy_dissipation: f64,
    /// Volume average of u dot f.
    pub forcing_work: f64,
    /// Volume average of omega dot S omega, using its periodic conservative identity.
    /// This identity requires divergence-free velocity; inspect `norms.divergence_l2`.
    pub stretching: f64,
    /// nu times the volume average of squared vorticity gradient.
    pub enstrophy_dissipation: f64,
    /// Volume average of omega dot curl(f).
    pub vorticity_forcing: f64,
}

impl BalanceSample {
    /// Exact balance values at zero velocity under a finite prescribed force.
    pub const REST: Self = Self {
        norms: Norms {
            l2: 0.0,
            h1: 0.0,
            vorticity_l2: 0.0,
            divergence_l2: 0.0,
        },
        energy: 0.0,
        enstrophy: 0.0,
        energy_dissipation: 0.0,
        forcing_work: 0.0,
        stretching: 0.0,
        enstrophy_dissipation: 0.0,
        vorticity_forcing: 0.0,
    };
}

/// Measure complete same-grid spectra. `conservative` is independently formed P(div(u tensor u)-f).
/// Curl removes the projection's pressure gradient. Thus -<omega dot curl(conservative+f)>
/// gives the periodic stretching integral without using a rotational stage equation.
pub fn measure(
    domain: Domain,
    velocity: [&[Complex64]; 3],
    force: [&[Complex64]; 3],
    conservative: [&[Complex64]; 3],
) -> Result<BalanceSample, SolverError> {
    let layout = domain.layout();
    for values in velocity.into_iter().chain(force).chain(conservative) {
        validate_spectrum(layout, values, 1e-12)?;
    }
    let mut sums = BalanceSums::default();
    for (index, _) in velocity[0].iter().enumerate() {
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            continue;
        }
        let k = modal::wavevector(domain, layout.mode(position)?)?;
        sums.push(
            k,
            std::array::from_fn(|axis| velocity[axis][index]),
            std::array::from_fn(|axis| force[axis][index]),
            std::array::from_fn(|axis| conservative[axis][index]),
            layout.weight(position)?,
        )?;
    }
    sums.finish(domain.viscosity())
}

#[derive(Default)]
struct BalanceSums {
    norms: NormSums,
    gradient_curl: Squares,
    work: Sum,
    stretching: Sum,
    vorticity_forcing: Sum,
}
impl BalanceSums {
    fn push(
        &mut self,
        k: [f64; 3],
        u: [Complex64; 3],
        f: [Complex64; 3],
        c: [Complex64; 3],
        weight: f64,
    ) -> Result<(), SolverError> {
        self.norms.push(k, u, weight)?;
        let omega = modal::curl(k, u)?;
        let curl_force = modal::curl(k, f)?;
        let curl_convection = modal::curl(k, std::array::from_fn(|axis| c[axis] + f[axis]))?;
        for axis in 0..3 {
            self.work.dot(u[axis], f[axis], weight)?;
            self.vorticity_forcing
                .dot(omega[axis], curl_force[axis], weight)?;
            self.stretching
                .dot(omega[axis], -curl_convection[axis], weight)?;
            for wave in k {
                self.gradient_curl.complex(wave * omega[axis], weight)?;
            }
        }
        Ok(())
    }

    fn finish(self, viscosity: f64) -> Result<BalanceSample, SolverError> {
        let norms = self.norms.finish()?;
        let gradient_curl = self.gradient_curl.norm()?;
        Ok(BalanceSample {
            norms,
            energy: finite(0.5 * norms.l2 * norms.l2)?,
            enstrophy: finite(0.5 * norms.vorticity_l2 * norms.vorticity_l2)?,
            energy_dissipation: finite(viscosity * norms.vorticity_l2 * norms.vorticity_l2)?,
            forcing_work: self.work.finish()?,
            stretching: self.stretching.finish()?,
            enstrophy_dissipation: finite(viscosity * gradient_curl * gradient_curl)?,
            vorticity_forcing: self.vorticity_forcing.finish()?,
        })
    }
}
