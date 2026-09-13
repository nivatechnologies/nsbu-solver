use crate::{
    diagnostics::norms::{CrossSums, NormSums, Norms, SignedNormChannels},
    Complex64, SolverError,
};

pub(super) type Terms = [[Complex64; 3]; 7];

/// Four norm-channel values used for cancellation ratios and identity errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormChannels {
    /// L2 channel.
    pub l2: f64,
    /// H1 channel.
    pub h1: f64,
    /// Curl L2 channel.
    pub vorticity_l2: f64,
    /// Divergence L2 channel.
    pub divergence_l2: f64,
}

/// Cancellation ratios, absent when all denominator terms vanish.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CancellationChannels {
    /// L2 cancellation ratio.
    pub l2: Option<f64>,
    /// H1 cancellation ratio.
    pub h1: Option<f64>,
    /// Curl L2 cancellation ratio.
    pub vorticity_l2: Option<f64>,
    /// Divergence L2 cancellation ratio.
    pub divergence_l2: Option<f64>,
}

/// Per-band decomposition of the M768 residual and the retained-force control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResidualBandLocalization {
    /// Reconstructed physical-time derivative D.
    pub derivative: Norms,
    /// PDE-side viscous term V.
    pub viscous: Norms,
    /// M768 conservative term C768.
    pub conservative_m768: Norms,
    /// Base M768 residual R768.
    pub residual_m768: Norms,
    /// Projected force-resolution delta.
    pub projected_force_delta: Norms,
    /// Retained-force conservative control C384.
    pub conservative_m384: Norms,
    /// Retained-force residual control R384.
    pub residual_m384: Norms,
    /// Pair order is D/V, D/C, V/C.
    pub base_cross: [SignedNormChannels; 3],
    /// Pair order is D/V, D/C, V/C, using C384.
    pub control_cross: [SignedNormChannels; 3],
    /// Normalized pair alignments for D/V, D/C768, and V/C768.
    pub base_alignment: [CancellationChannels; 3],
    /// Normalized pair alignments for D/V, D/C384, and V/C384.
    pub control_alignment: [CancellationChannels; 3],
    /// Cancellation ratios for D+V+C768.
    pub base_cancellation: CancellationChannels,
    /// Cancellation ratios for D+V+C384.
    pub control_cancellation: CancellationChannels,
    /// Relative squared-norm identity errors for the base decomposition.
    pub base_identity_relative_error: NormChannels,
    /// Relative squared-norm identity errors for the control decomposition.
    pub control_identity_relative_error: NormChannels,
}

#[derive(Default)]
pub(super) struct BandSums {
    terms: [NormSums; 7],
    base_cross: [CrossSums; 3],
    control_cross: [CrossSums; 3],
}

impl BandSums {
    pub(super) fn push(
        &mut self,
        k: [f64; 3],
        terms: Terms,
        weight: f64,
    ) -> Result<(), SolverError> {
        for (sum, term) in self.terms.iter_mut().zip(terms) {
            sum.push(k, term, weight)?;
        }
        for (sum, (left, right)) in self.base_cross.iter_mut().zip([(0, 1), (0, 2), (1, 2)]) {
            sum.push(k, terms[left], terms[right], weight)?;
        }
        for (sum, (left, right)) in self.control_cross.iter_mut().zip([(0, 1), (0, 5), (1, 5)]) {
            sum.push(k, terms[left], terms[right], weight)?;
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<ResidualBandLocalization, SolverError> {
        let mut terms = self.terms.into_iter().map(NormSums::finish);
        let derivative = terms.next().ok_or(SolverError::InvalidPayload)??;
        let viscous = terms.next().ok_or(SolverError::InvalidPayload)??;
        let conservative_m768 = terms.next().ok_or(SolverError::InvalidPayload)??;
        let residual_m768 = terms.next().ok_or(SolverError::InvalidPayload)??;
        let projected_force_delta = terms.next().ok_or(SolverError::InvalidPayload)??;
        let conservative_m384 = terms.next().ok_or(SolverError::InvalidPayload)??;
        let residual_m384 = terms.next().ok_or(SolverError::InvalidPayload)??;
        let base_cross = finish_cross(self.base_cross)?;
        let control_cross = finish_cross(self.control_cross)?;
        ResidualBandLocalization::new(
            derivative,
            viscous,
            conservative_m768,
            residual_m768,
            projected_force_delta,
            conservative_m384,
            residual_m384,
            base_cross,
            control_cross,
        )
    }
}

impl ResidualBandLocalization {
    #[allow(clippy::too_many_arguments)]
    fn new(
        derivative: Norms,
        viscous: Norms,
        conservative_m768: Norms,
        residual_m768: Norms,
        projected_force_delta: Norms,
        conservative_m384: Norms,
        residual_m384: Norms,
        base_cross: [SignedNormChannels; 3],
        control_cross: [SignedNormChannels; 3],
    ) -> Result<Self, SolverError> {
        Ok(Self {
            derivative,
            viscous,
            conservative_m768,
            residual_m768,
            projected_force_delta,
            conservative_m384,
            residual_m384,
            base_cross,
            control_cross,
            base_alignment: alignments(derivative, viscous, conservative_m768, base_cross)?,
            control_alignment: alignments(derivative, viscous, conservative_m384, control_cross)?,
            base_cancellation: cancellation(derivative, viscous, conservative_m768, residual_m768)?,
            control_cancellation: cancellation(
                derivative,
                viscous,
                conservative_m384,
                residual_m384,
            )?,
            base_identity_relative_error: identity_error(
                derivative,
                viscous,
                conservative_m768,
                residual_m768,
                base_cross,
            )?,
            control_identity_relative_error: identity_error(
                derivative,
                viscous,
                conservative_m384,
                residual_m384,
                control_cross,
            )?,
        })
    }

    pub(super) fn orthogonal_sum(left: Self, right: Self) -> Result<Self, SolverError> {
        Self::new(
            left.derivative.orthogonal_sum(right.derivative)?,
            left.viscous.orthogonal_sum(right.viscous)?,
            left.conservative_m768
                .orthogonal_sum(right.conservative_m768)?,
            left.residual_m768.orthogonal_sum(right.residual_m768)?,
            left.projected_force_delta
                .orthogonal_sum(right.projected_force_delta)?,
            left.conservative_m384
                .orthogonal_sum(right.conservative_m384)?,
            left.residual_m384.orthogonal_sum(right.residual_m384)?,
            add_cross(left.base_cross, right.base_cross),
            add_cross(left.control_cross, right.control_cross),
        )
    }
}

fn add_cross(
    left: [SignedNormChannels; 3],
    right: [SignedNormChannels; 3],
) -> [SignedNormChannels; 3] {
    std::array::from_fn(|i| SignedNormChannels {
        l2: left[i].l2 + right[i].l2,
        h1: left[i].h1 + right[i].h1,
        vorticity_l2: left[i].vorticity_l2 + right[i].vorticity_l2,
        divergence_l2: left[i].divergence_l2 + right[i].divergence_l2,
    })
}

fn alignments(
    a: Norms,
    b: Norms,
    c: Norms,
    cross: [SignedNormChannels; 3],
) -> Result<[CancellationChannels; 3], SolverError> {
    let terms = [(a, b), (a, c), (b, c)];
    let mut output = [empty_cancellation(); 3];
    for (index, ((left, right), value)) in terms.into_iter().zip(cross).enumerate() {
        output[index] = alignment(left, right, value)?;
    }
    Ok(output)
}

fn alignment(
    left: Norms,
    right: Norms,
    value: SignedNormChannels,
) -> Result<CancellationChannels, SolverError> {
    let output = CancellationChannels {
        l2: alignment_ratio(left.l2, right.l2, value.l2),
        h1: alignment_ratio(left.h1, right.h1, value.h1),
        vorticity_l2: alignment_ratio(left.vorticity_l2, right.vorticity_l2, value.vorticity_l2),
        divergence_l2: alignment_ratio(
            left.divergence_l2,
            right.divergence_l2,
            value.divergence_l2,
        ),
    };
    ensure_finite_options(output)?;
    Ok(output)
}

fn alignment_ratio(x: f64, y: f64, z: f64) -> Option<f64> {
    if x == 0.0 || y == 0.0 {
        None
    } else {
        Some(z / (2.0 * x * y))
    }
}

fn empty_cancellation() -> CancellationChannels {
    CancellationChannels {
        l2: None,
        h1: None,
        vorticity_l2: None,
        divergence_l2: None,
    }
}

fn ensure_finite_options(values: CancellationChannels) -> Result<(), SolverError> {
    if [
        values.l2,
        values.h1,
        values.vorticity_l2,
        values.divergence_l2,
    ]
    .into_iter()
    .flatten()
    .any(|number| !number.is_finite())
    {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    Ok(())
}

fn finish_cross(sums: [CrossSums; 3]) -> Result<[SignedNormChannels; 3], SolverError> {
    let mut finished = sums.into_iter().map(CrossSums::finish);
    Ok([
        finished.next().ok_or(SolverError::InvalidPayload)??,
        finished.next().ok_or(SolverError::InvalidPayload)??,
        finished.next().ok_or(SolverError::InvalidPayload)??,
    ])
}

fn cancellation(
    a: Norms,
    b: Norms,
    c: Norms,
    r: Norms,
) -> Result<CancellationChannels, SolverError> {
    let values = CancellationChannels {
        l2: cancellation_ratio(a.l2, b.l2, c.l2, r.l2),
        h1: cancellation_ratio(a.h1, b.h1, c.h1, r.h1),
        vorticity_l2: cancellation_ratio(
            a.vorticity_l2,
            b.vorticity_l2,
            c.vorticity_l2,
            r.vorticity_l2,
        ),
        divergence_l2: cancellation_ratio(
            a.divergence_l2,
            b.divergence_l2,
            c.divergence_l2,
            r.divergence_l2,
        ),
    };
    ensure_finite_options(values)?;
    Ok(values)
}

fn cancellation_ratio(x: f64, y: f64, z: f64, result: f64) -> Option<f64> {
    let denominator = x + y + z;
    if denominator == 0.0 {
        None
    } else {
        Some(result / denominator)
    }
}

fn identity_error(
    a: Norms,
    b: Norms,
    c: Norms,
    r: Norms,
    cross: [SignedNormChannels; 3],
) -> Result<NormChannels, SolverError> {
    channels([
        relative(a.l2, b.l2, c.l2, r.l2, cross.map(|v| v.l2)),
        relative(a.h1, b.h1, c.h1, r.h1, cross.map(|v| v.h1)),
        relative(
            a.vorticity_l2,
            b.vorticity_l2,
            c.vorticity_l2,
            r.vorticity_l2,
            cross.map(|v| v.vorticity_l2),
        ),
        relative(
            a.divergence_l2,
            b.divergence_l2,
            c.divergence_l2,
            r.divergence_l2,
            cross.map(|v| v.divergence_l2),
        ),
    ])
}

fn relative(x: f64, y: f64, z: f64, result: f64, cross: [f64; 3]) -> f64 {
    let expected = x * x + y * y + z * z + cross.into_iter().sum::<f64>();
    let actual = result * result;
    let scale = actual.abs().max(expected.abs()).max(1.0);
    (actual - expected).abs() / scale
}

fn channels(values: [f64; 4]) -> Result<NormChannels, SolverError> {
    if values.into_iter().any(|value| !value.is_finite()) {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    Ok(NormChannels {
        l2: values[0],
        h1: values[1],
        vorticity_l2: values[2],
        divergence_l2: values[3],
    })
}
