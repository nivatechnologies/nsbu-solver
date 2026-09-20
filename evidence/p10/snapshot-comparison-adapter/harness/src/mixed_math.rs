//! Allocation-free norm and cross-term arithmetic for the mixed diagnostic.
//!
//! The band accumulators and compensated arithmetic live in the [`kernels`]
//! submodule; this module keeps the published metric types, the closed
//! domain/value validation and the mode-loop entry point.

pub(crate) mod kernels;

use crate::model::debug;
use kernels::MixedSums;
use nsbu_solver::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct MixedNorms {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct SplitNorms {
    pub full: MixedNorms,
    pub common: MixedNorms,
    pub newly_resolved: MixedNorms,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct CrossChannels {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct CosineSimilarityChannels {
    pub l2: Option<f64>,
    pub h1: Option<f64>,
    pub vorticity_l2: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct CrossOutput {
    pub twice_real_inner_product: CrossChannels,
    pub cosine_similarity: CosineSimilarityChannels,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct SplitCrossOutput {
    pub full: CrossOutput,
    pub common: CrossOutput,
    pub newly_resolved: CrossOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct MixedMetrics {
    /// `U384M384 - lift(U256M384)`.
    pub spatial_a: SplitNorms,
    /// `U384M512 - U384M384`.
    pub force_b: SplitNorms,
    /// `U384M512 - lift(U256M384) = A + B`.
    pub combined_c: SplitNorms,
    /// Weighted `2 Re <A,B>` and the corresponding cosine similarity.
    pub cross_a_b: SplitCrossOutput,
}

pub(crate) fn calculate(
    coarse_domain: Domain,
    coarse: [&[Complex64]; 3],
    baseline_domain: Domain,
    baseline: [&[Complex64]; 3],
    force_domain: Domain,
    force: [&[Complex64]; 3],
) -> Result<MixedMetrics, String> {
    validate_domains(coarse_domain, baseline_domain, force_domain)?;
    validate_values(coarse_domain, coarse)?;
    validate_values(baseline_domain, baseline)?;
    validate_values(force_domain, force)?;
    let mut sums = MixedSums::default();
    let coarse_layout = coarse_domain.layout();
    let fine_layout = baseline_domain.layout();
    for (index, _) in baseline[0].iter().enumerate() {
        let position = fine_layout.position(index).map_err(debug)?;
        if fine_layout.is_nyquist(position).map_err(debug)? {
            continue;
        }
        let mode = fine_layout.mode(position).map_err(debug)?;
        let coarse_index = coarse_layout.locate(mode).ok().map(|(index, _)| index);
        let lifted: [Complex64; 3] = std::array::from_fn(|axis| {
            coarse_index.map_or(Complex64::new(0.0, 0.0), |i| coarse[axis][i])
        });
        let base: [Complex64; 3] = std::array::from_fn(|axis| baseline[axis][index]);
        let forced: [Complex64; 3] = std::array::from_fn(|axis| force[axis][index]);
        let a = std::array::from_fn(|axis| base[axis] - lifted[axis]);
        let b = std::array::from_fn(|axis| forced[axis] - base[axis]);
        let c = std::array::from_fn(|axis| forced[axis] - lifted[axis]);
        sums.push(
            coarse_index.is_some(),
            modal::wavevector(baseline_domain, mode).map_err(debug)?,
            a,
            b,
            c,
            fine_layout.weight(position).map_err(debug)?,
        )?;
    }
    sums.finish()
}

fn validate_domains(coarse: Domain, baseline: Domain, force: Domain) -> Result<(), String> {
    if coarse.lengths() != baseline.lengths()
        || baseline.lengths() != force.lengths()
        || coarse.viscosity() != baseline.viscosity()
        || baseline.viscosity() != force.viscosity()
        || coarse
            .layout()
            .dimensions()
            .into_iter()
            .zip(baseline.layout().dimensions())
            .any(|(coarse, fine)| coarse > fine)
        || baseline.layout() != force.layout()
    {
        return Err("mixed diagnostic domain mismatch".into());
    }
    Ok(())
}

fn validate_values(domain: Domain, values: [&[Complex64]; 3]) -> Result<(), String> {
    for component in values {
        validate_spectrum(domain.layout(), component, 1e-12).map_err(debug)?;
    }
    Ok(())
}
