use nsbu_solver::{Complex64, domain::Layout};
use std::f64::consts::TAU;

#[derive(Clone, Copy)]
pub struct Norms {
    pub l2: f64,
    pub h1: f64,
    pub vorticity: f64,
    pub divergence: f64,
}

pub struct HighBandShells {
    layout: Layout,
    dot: Vec<Complex64>,
    l2: [f64; 2],
    gradient: [f64; 2],
}

impl HighBandShells {
    pub fn new(layout: Layout) -> Result<Self, String> {
        validate_layout(layout)?;
        Ok(Self {
            layout,
            dot: vec![Complex64::new(0.0, 0.0); layout.half_len()],
            l2: [0.0; 2],
            gradient: [0.0; 2],
        })
    }

    pub fn add(
        &mut self,
        component: usize,
        index: usize,
        position: [usize; 3],
        value: Complex64,
    ) -> Result<(), String> {
        if component >= 3 {
            return Err("raw shell component mismatch".into());
        }
        if self.layout.is_nyquist(position).map_err(debug)? {
            return Ok(());
        }
        let mode = self.layout.mode(position).map_err(debug)?;
        let Some(bucket) = bucket(mode) else {
            return Ok(());
        };
        let weight = self.layout.weight(position).map_err(debug)?;
        if weight == 0.0 {
            return Err("non-Nyquist shell mode has zero weight".into());
        }
        let k = mode.map(|x| TAU * x as f64);
        let energy = value.norm_sqr();
        let k2 = k.iter().map(|x| x * x).sum::<f64>();
        self.l2[bucket] += weight * energy;
        self.gradient[bucket] += weight * k2 * energy;
        self.dot[index] += value * k[component];
        Ok(())
    }

    pub fn finish(self) -> Result<[(&'static str, Norms); 2], String> {
        let mut divergence = [0.0; 2];
        for (index, dot) in self.dot.iter().enumerate() {
            let position = self.layout.position(index).map_err(debug)?;
            if self.layout.is_nyquist(position).map_err(debug)? {
                continue;
            }
            let Some(bucket) = bucket(self.layout.mode(position).map_err(debug)?) else {
                continue;
            };
            divergence[bucket] += self.layout.weight(position).map_err(debug)? * dot.norm_sqr();
        }
        Ok([
            (
                "n384_to_n512",
                norms(self.l2[0], self.gradient[0], divergence[0])?,
            ),
            (
                "n512_to_n768",
                norms(self.l2[1], self.gradient[1], divergence[1])?,
            ),
        ])
    }
}

fn bucket(mode: [isize; 3]) -> Option<usize> {
    let radius = mode.iter().map(|x| x.unsigned_abs()).max()?;
    if radius < 192 {
        None
    } else if radius < 256 {
        Some(0)
    } else if radius < 384 {
        Some(1)
    } else {
        None
    }
}

fn norms(l2: f64, gradient: f64, divergence: f64) -> Result<Norms, String> {
    let scale = gradient.max(divergence).max(1.0);
    let vorticity = gradient - divergence;
    if !l2.is_finite()
        || !gradient.is_finite()
        || !divergence.is_finite()
        || vorticity < -128.0 * f64::EPSILON * scale
    {
        return Err("nonfinite or inconsistent raw shell norm".into());
    }
    Ok(Norms {
        l2: l2.sqrt(),
        h1: (l2 + gradient).sqrt(),
        vorticity: vorticity.max(0.0).sqrt(),
        divergence: divergence.sqrt(),
    })
}

fn validate_layout(layout: Layout) -> Result<(), String> {
    if layout.dimensions() != [768; 3] {
        return Err("raw shell audit requires M768".into());
    }
    for (mode, expected) in [
        ([191, 0, 0], None),
        ([192, 0, 0], Some(0)),
        ([255, 0, 0], Some(0)),
        ([256, 0, 0], Some(1)),
        ([-256, 0, 0], Some(1)),
        ([383, 0, 0], Some(1)),
        ([384, 0, 0], None),
    ] {
        if bucket(mode) != expected {
            return Err("raw shell boundary mismatch".into());
        }
    }
    for position in [[384, 0, 0], [0, 384, 0], [0, 0, 384]] {
        if !layout.is_nyquist(position).map_err(debug)?
            || layout.weight(position).map_err(debug)? != 0.0
            || bucket(layout.mode(position).map_err(debug)?).is_some()
        {
            return Err("raw shell Nyquist mismatch".into());
        }
    }
    if layout.weight([0, 0, 0]).map_err(debug)? != 1.0
        || layout.weight([0, 0, 1]).map_err(debug)? != 2.0
    {
        return Err("raw shell Parseval weight mismatch".into());
    }
    Ok(())
}

fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundaries_nyquist_and_weights_are_admitted() {
        validate_layout(Layout::new([768; 3]).unwrap()).unwrap();
    }
}
