//! Allocation-free absolute norm producer mirroring diagnostics/norms.rs operation order.
use crate::model::{debug, NormOutput};
use nsbu_solver::{domain::Domain, spectral::modal, Complex64};

pub(crate) fn measure(domain: Domain, values: [&[Complex64]; 3]) -> Result<NormOutput, String> {
    let layout = domain.layout();
    let mut sums = NormSums::default();
    for (index, _) in values[0].iter().enumerate() {
        let position = layout.position(index).map_err(debug)?;
        if layout.is_nyquist(position).map_err(debug)? {
            continue;
        }
        let wave =
            modal::wavevector(domain, layout.mode(position).map_err(debug)?).map_err(debug)?;
        sums.push(
            wave,
            std::array::from_fn(|axis| values[axis][index]),
            layout.weight(position).map_err(debug)?,
        )?;
    }
    sums.finish()
}

#[derive(Default)]
struct NormSums {
    l2: Squares,
    h1: Squares,
    curl: Squares,
    divergence: Squares,
}

impl NormSums {
    fn push(&mut self, k: [f64; 3], u: [Complex64; 3], weight: f64) -> Result<(), String> {
        for value in u {
            self.l2.complex(value, weight)?;
            self.h1.complex(value, weight)?;
            for frequency in k {
                self.h1.complex(value * frequency, weight)?;
            }
        }
        for (next, last) in [(1, 2), (2, 0), (0, 1)] {
            self.curl
                .complex(k[next] * u[last] - k[last] * u[next], weight)?;
        }
        self.divergence
            .complex(k[0] * u[0] + k[1] * u[1] + k[2] * u[2], weight)
    }

    fn finish(self) -> Result<NormOutput, String> {
        Ok(NormOutput {
            l2: self.l2.norm()?,
            h1: self.h1.norm()?,
            vorticity_l2: self.curl.norm()?,
            divergence_l2: self.divergence.norm()?,
        })
    }
}

#[derive(Default)]
struct Squares {
    scale: f64,
    sum: f64,
}

impl Squares {
    fn complex(&mut self, value: Complex64, weight: f64) -> Result<(), String> {
        self.push(value.re, weight)?;
        self.push(value.im, weight)
    }

    fn push(&mut self, value: f64, weight: f64) -> Result<(), String> {
        let value = value.abs();
        if !value.is_finite() {
            return Err("absolute norm arithmetic is nonfinite".into());
        }
        if value == 0.0 {
            return Ok(());
        }
        let scale = self.scale.max(value);
        self.sum = self.sum * (self.scale / scale).powi(2) + weight * (value / scale).powi(2);
        self.scale = scale;
        Ok(())
    }

    fn norm(self) -> Result<f64, String> {
        let result = self.scale * self.sum.sqrt();
        if result.is_finite() {
            Ok(result)
        } else {
            Err("absolute norm arithmetic is nonfinite".into())
        }
    }
}
