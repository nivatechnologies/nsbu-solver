//! Band accumulators for the mixed diagnostic.
//!
//! Every function here is a byte-for-byte relocation from the original
//! `mixed_math` module: same conjuncts, same floating-point operation order,
//! same refusal messages. The module boundary is purely organizational.

mod primitives;

use super::{
    CosineSimilarityChannels, CrossChannels, CrossOutput, MixedMetrics, MixedNorms,
    SplitCrossOutput, SplitNorms,
};
use nsbu_solver::Complex64;
use primitives::{curl, finite, real_inner, Compensated, ScaledSquares};

#[derive(Default)]
pub(super) struct MixedSums {
    common: BandSums,
    newly: BandSums,
}

impl MixedSums {
    pub(super) fn push(
        &mut self,
        common: bool,
        wave: [f64; 3],
        a: [Complex64; 3],
        b: [Complex64; 3],
        c: [Complex64; 3],
        weight: f64,
    ) -> Result<(), String> {
        let target = if common {
            &mut self.common
        } else {
            &mut self.newly
        };
        target.push(wave, a, b, c, weight)
    }

    pub(super) fn finish(self) -> Result<MixedMetrics, String> {
        let common = self.common.finish()?;
        let newly = self.newly.finish()?;
        let spatial_a = split(common.a, newly.a)?;
        let force_b = split(common.b, newly.b)?;
        let combined_c = split(common.c, newly.c)?;
        Ok(MixedMetrics {
            spatial_a,
            force_b,
            combined_c,
            cross_a_b: SplitCrossOutput {
                full: cross_output(
                    add_cross(common.cross, newly.cross)?,
                    spatial_a.full,
                    force_b.full,
                )?,
                common: cross_output(common.cross, spatial_a.common, force_b.common)?,
                newly_resolved: cross_output(
                    newly.cross,
                    spatial_a.newly_resolved,
                    force_b.newly_resolved,
                )?,
            },
        })
    }
}

#[derive(Default)]
struct BandSums {
    a: NormSums,
    b: NormSums,
    c: NormSums,
    cross: CrossSums,
}

impl BandSums {
    fn push(
        &mut self,
        wave: [f64; 3],
        a: [Complex64; 3],
        b: [Complex64; 3],
        c: [Complex64; 3],
        weight: f64,
    ) -> Result<(), String> {
        self.a.push(wave, a, weight)?;
        self.b.push(wave, b, weight)?;
        self.c.push(wave, c, weight)?;
        self.cross.push(wave, a, b, weight)
    }

    fn finish(self) -> Result<BandValues, String> {
        Ok(BandValues {
            a: self.a.finish()?,
            b: self.b.finish()?,
            c: self.c.finish()?,
            cross: self.cross.finish()?,
        })
    }
}

#[derive(Clone, Copy)]
struct BandValues {
    a: MixedNorms,
    b: MixedNorms,
    c: MixedNorms,
    cross: CrossChannels,
}

#[derive(Default)]
struct NormSums {
    l2: ScaledSquares,
    h1: ScaledSquares,
    curl: ScaledSquares,
}

impl NormSums {
    fn push(&mut self, wave: [f64; 3], value: [Complex64; 3], weight: f64) -> Result<(), String> {
        for component in value {
            self.l2.complex(component, weight)?;
            self.h1.complex(component, weight)?;
            for frequency in wave {
                self.h1.complex(component * frequency, weight)?;
            }
        }
        for component in curl(wave, value) {
            self.curl.complex(component, weight)?;
        }
        Ok(())
    }

    fn finish(self) -> Result<MixedNorms, String> {
        Ok(MixedNorms {
            l2: self.l2.norm()?,
            h1: self.h1.norm()?,
            vorticity_l2: self.curl.norm()?,
        })
    }
}

#[derive(Default)]
struct CrossSums {
    l2: Compensated,
    h1: Compensated,
    curl: Compensated,
}

impl CrossSums {
    fn push(
        &mut self,
        wave: [f64; 3],
        a: [Complex64; 3],
        b: [Complex64; 3],
        weight: f64,
    ) -> Result<(), String> {
        let scale = 2.0 * weight;
        let k_squared = wave.into_iter().map(|value| value * value).sum::<f64>();
        for axis in 0..3 {
            let value = scale * real_inner(a[axis], b[axis]);
            self.l2.push(value)?;
            self.h1.push(value * (1.0 + k_squared))?;
        }
        for (a, b) in curl(wave, a).into_iter().zip(curl(wave, b)) {
            self.curl.push(scale * real_inner(a, b))?;
        }
        Ok(())
    }

    fn finish(self) -> Result<CrossChannels, String> {
        Ok(CrossChannels {
            l2: self.l2.finish()?,
            h1: self.h1.finish()?,
            vorticity_l2: self.curl.finish()?,
        })
    }
}

fn split(common: MixedNorms, newly_resolved: MixedNorms) -> Result<SplitNorms, String> {
    Ok(SplitNorms {
        full: add_norms(common, newly_resolved)?,
        common,
        newly_resolved,
    })
}

fn add_norms(left: MixedNorms, right: MixedNorms) -> Result<MixedNorms, String> {
    Ok(MixedNorms {
        l2: finite(left.l2.hypot(right.l2))?,
        h1: finite(left.h1.hypot(right.h1))?,
        vorticity_l2: finite(left.vorticity_l2.hypot(right.vorticity_l2))?,
    })
}

fn add_cross(left: CrossChannels, right: CrossChannels) -> Result<CrossChannels, String> {
    Ok(CrossChannels {
        l2: finite(left.l2 + right.l2)?,
        h1: finite(left.h1 + right.h1)?,
        vorticity_l2: finite(left.vorticity_l2 + right.vorticity_l2)?,
    })
}

fn cross_output(cross: CrossChannels, a: MixedNorms, b: MixedNorms) -> Result<CrossOutput, String> {
    Ok(CrossOutput {
        twice_real_inner_product: cross,
        cosine_similarity: CosineSimilarityChannels {
            l2: cosine_similarity(cross.l2, a.l2, b.l2)?,
            h1: cosine_similarity(cross.h1, a.h1, b.h1)?,
            vorticity_l2: cosine_similarity(cross.vorticity_l2, a.vorticity_l2, b.vorticity_l2)?,
        },
    })
}

fn cosine_similarity(twice_inner: f64, left: f64, right: f64) -> Result<Option<f64>, String> {
    if left == 0.0 || right == 0.0 {
        return Ok(None);
    }
    Ok(Some(finite((twice_inner * 0.5 / left) / right)?))
}
