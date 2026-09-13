//! Full-double-band assembly of reconstructed physical-time PDE defects.
use super::{
    conservative::ConservativeWorkspace,
    norms::{CrossSums, NormSums, Norms, SignedNormChannels},
};
use crate::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};

/// Geometry for borrowed residual assembly; owns no storage and never alters a trajectory.
#[derive(Debug, Clone, Copy)]
pub struct ResidualPlan {
    source: Domain,
    diagnostic: Domain,
}

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

/// Strict N384/new-shell localization over one complete N768 residual traversal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResidualLocalization {
    /// Modes in the strict N384 retained band.
    pub retained_strict_n384: ResidualBandLocalization,
    /// Non-Nyquist N768 modes outside the strict N384 band.
    pub new_shell_n768: ResidualBandLocalization,
    /// Orthogonal sum of retained and shell bands.
    pub full_n768: ResidualBandLocalization,
    /// Number of visited strict N384 modes.
    pub retained_modes: usize,
    /// Number of visited new-shell modes.
    pub new_shell_modes: usize,
    /// Number of excluded N768 Nyquist storage slots.
    pub excluded_nyquist_slots: usize,
    /// Per-component hashes of projected force delta, in coefficient order.
    pub projected_force_delta_component_sha256: [[u8; 32]; 3],
    /// Per-component hashes of R384, in coefficient order.
    pub residual_m384_component_sha256: [[u8; 32]; 3],
}
impl ResidualPlan {
    /// Admit the retained and doubled geometries. FFT workspace admission is separate.
    pub fn new(source: Domain) -> Result<Self, SolverError> {
        Ok(Self {
            source,
            diagnostic: ConservativeWorkspace::diagnostic_domain(source)?,
        })
    }

    /// Assemble v_t + P div(v tensor v) - nu Delta v - P f on the complete double grid.
    /// `conservative` must be independently evaluated at the same probe and represents
    /// P(div(v tensor v) - f). Returned norms are samples, not time-supremum bounds.
    /// Errors invalidate output scratch; no reference field is substituted into an integrated state.
    pub fn evaluate(
        self,
        velocity: [&[Complex64]; 3],
        derivative: [&[Complex64]; 3],
        conservative: [&[Complex64]; 3],
        output: [&mut [Complex64]; 3],
    ) -> Result<Norms, SolverError> {
        for values in velocity.into_iter().chain(derivative) {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        for values in conservative {
            validate_spectrum(self.diagnostic.layout(), values, 1e-12)?;
        }
        let layout = self.diagnostic.layout();
        if output
            .iter()
            .any(|values| values.len() != layout.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let mut norms = NormSums::default();
        let [a, b, c] = output;
        for (index, ((a, b), c)) in a.iter_mut().zip(b).zip(c).enumerate() {
            let position = layout.position(index)?;
            if layout.is_nyquist(position)? {
                *a = Complex64::new(0.0, 0.0);
                *b = *a;
                *c = *a;
                continue;
            }
            let mode = layout.mode(position)?;
            let k = modal::wavevector(self.diagnostic, mode)?;
            let mut value = std::array::from_fn(|axis| conservative[axis][index]);
            if let Ok((source_index, _)) = self.source.layout().locate(mode) {
                for (axis, value) in value.iter_mut().enumerate() {
                    *value += derivative[axis][source_index];
                    for wave in k {
                        *value +=
                            self.source.viscosity() * wave * (wave * velocity[axis][source_index]);
                    }
                }
            }
            norms.push(k, value, layout.weight(position)?)?;
            [*a, *b, *c] = value;
        }
        norms.finish()
    }

    /// Assemble the M768 residual while localizing strict retained and new-shell terms.
    /// The retained-force control uses `P(f768-pad(f384))` and owns no diagnostic field.
    pub fn evaluate_localized(
        self,
        velocity: [&[Complex64]; 3],
        derivative: [&[Complex64]; 3],
        conservative_m768: [&[Complex64]; 3],
        force_m768: [&[Complex64]; 3],
        force_m384: [&[Complex64]; 3],
        output_m768: [&mut [Complex64]; 3],
    ) -> Result<ResidualLocalization, SolverError> {
        for values in velocity.into_iter().chain(derivative).chain(force_m384) {
            validate_spectrum(self.source.layout(), values, 1e-12)?;
        }
        for values in conservative_m768.into_iter().chain(force_m768) {
            validate_spectrum(self.diagnostic.layout(), values, 1e-12)?;
        }
        let layout = self.diagnostic.layout();
        if output_m768
            .iter()
            .any(|values| values.len() != layout.half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        let mut retained = BandSums::default();
        let mut shell = BandSums::default();
        let mut retained_modes = 0usize;
        let mut new_shell_modes = 0usize;
        let mut excluded_nyquist_slots = 0usize;
        let mut delta_hashes: [Sha256; 3] = std::array::from_fn(|_| Sha256::new());
        let mut control_hashes: [Sha256; 3] = std::array::from_fn(|_| Sha256::new());
        let [a, b, c] = output_m768;
        for (index, ((a, b), c)) in a.iter_mut().zip(b).zip(c).enumerate() {
            let position = layout.position(index)?;
            if layout.is_nyquist(position)? {
                *a = Complex64::new(0.0, 0.0);
                *b = *a;
                *c = *a;
                for hash in delta_hashes.iter_mut().chain(&mut control_hashes) {
                    hash.update(0.0_f64.to_bits().to_le_bytes());
                    hash.update(0.0_f64.to_bits().to_le_bytes());
                }
                excluded_nyquist_slots += 1;
                continue;
            }
            let mode = layout.mode(position)?;
            let k = modal::wavevector(self.diagnostic, mode)?;
            let located = self.source.layout().locate(mode).ok().map(|(i, _)| i);
            let zero = Complex64::new(0.0, 0.0);
            let d = std::array::from_fn(|axis| located.map_or(zero, |i| derivative[axis][i]));
            let mut v = [zero; 3];
            let mut r = std::array::from_fn(|axis| conservative_m768[axis][index]);
            for axis in 0..3 {
                r[axis] += d[axis];
                if let Some(source_index) = located {
                    for wave in k {
                        let contribution = self.source.viscosity()
                            * wave
                            * (wave * velocity[axis][source_index]);
                        v[axis] += contribution;
                        r[axis] += contribution;
                    }
                }
            }
            let raw_force_delta = std::array::from_fn(|axis| {
                force_m768[axis][index]
                    - located.map_or(zero, |i| force_m384[axis][i])
            });
            let projected_force_delta = modal::project(k, raw_force_delta)?;
            let conservative_control = std::array::from_fn(|axis| {
                conservative_m768[axis][index] + projected_force_delta[axis]
            });
            let residual_control =
                std::array::from_fn(|axis| r[axis] + projected_force_delta[axis]);
            for axis in 0..3 {
                for (hash, value) in [
                    (&mut delta_hashes[axis], projected_force_delta[axis]),
                    (&mut control_hashes[axis], residual_control[axis]),
                ] {
                    hash.update(value.re.to_bits().to_le_bytes());
                    hash.update(value.im.to_bits().to_le_bytes());
                }
            }
            [*a, *b, *c] = r;
            let terms = [
                d,
                v,
                std::array::from_fn(|axis| conservative_m768[axis][index]),
                r,
                projected_force_delta,
                conservative_control,
                residual_control,
            ];
            let sums = if located.is_some() {
                retained_modes += 1;
                &mut retained
            } else {
                new_shell_modes += 1;
                if r != terms[2] || d != [zero; 3] || v != [zero; 3] {
                    return Err(SolverError::InvalidSpectrum);
                }
                &mut shell
            };
            sums.push(k, terms, layout.weight(position)?)?;
        }
        let retained = retained.finish()?;
        let shell = shell.finish()?;
        let full = ResidualBandLocalization::orthogonal_sum(retained, shell)?;
        Ok(ResidualLocalization {
            retained_strict_n384: retained,
            new_shell_n768: shell,
            full_n768: full,
            retained_modes,
            new_shell_modes,
            excluded_nyquist_slots,
            projected_force_delta_component_sha256: delta_hashes.map(|hash| hash.finalize().into()),
            residual_m384_component_sha256: control_hashes.map(|hash| hash.finalize().into()),
        })
    }
}

#[derive(Default)]
struct BandSums {
    terms: [NormSums; 7],
    base_cross: [CrossSums; 3],
    control_cross: [CrossSums; 3],
}

impl BandSums {
    fn push(
        &mut self,
        k: [f64; 3],
        terms: [[Complex64; 3]; 7],
        weight: f64,
    ) -> Result<(), SolverError> {
        for (sum, term) in self.terms.iter_mut().zip(terms) {
            sum.push(k, term, weight)?;
        }
        for (sum, (left, right)) in self.base_cross.iter_mut().zip([(0, 1), (0, 2), (1, 2)]) {
            sum.push(k, terms[left], terms[right], weight)?;
        }
        for (sum, (left, right)) in self
            .control_cross
            .iter_mut()
            .zip([(0, 1), (0, 5), (1, 5)])
        {
            sum.push(k, terms[left], terms[right], weight)?;
        }
        Ok(())
    }

    fn finish(self) -> Result<ResidualBandLocalization, SolverError> {
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

fn alignments(
    a: Norms,
    b: Norms,
    c: Norms,
    cross: [SignedNormChannels; 3],
) -> Result<[CancellationChannels; 3], SolverError> {
    let terms = [(a, b), (a, c), (b, c)];
    let mut output = [CancellationChannels {
        l2: None,
        h1: None,
        vorticity_l2: None,
        divergence_l2: None,
    }; 3];
    for (index, ((left, right), value)) in terms.into_iter().zip(cross).enumerate() {
        let ratio = |x: f64, y: f64, z: f64| {
            if x == 0.0 || y == 0.0 {
                None
            } else {
                Some(z / (2.0 * x * y))
            }
        };
        output[index] = CancellationChannels {
            l2: ratio(left.l2, right.l2, value.l2),
            h1: ratio(left.h1, right.h1, value.h1),
            vorticity_l2: ratio(
                left.vorticity_l2,
                right.vorticity_l2,
                value.vorticity_l2,
            ),
            divergence_l2: ratio(
                left.divergence_l2,
                right.divergence_l2,
                value.divergence_l2,
            ),
        };
        if [
            output[index].l2,
            output[index].h1,
            output[index].vorticity_l2,
            output[index].divergence_l2,
        ]
        .into_iter()
        .flatten()
        .any(|number| !number.is_finite())
        {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
    }
    Ok(output)
}

fn finish_cross(sums: [CrossSums; 3]) -> Result<[SignedNormChannels; 3], SolverError> {
    let mut finished = sums.into_iter().map(CrossSums::finish);
    Ok([
        finished.next().ok_or(SolverError::InvalidPayload)??,
        finished.next().ok_or(SolverError::InvalidPayload)??,
        finished.next().ok_or(SolverError::InvalidPayload)??,
    ])
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
            base_alignment: alignments(
                derivative,
                viscous,
                conservative_m768,
                base_cross,
            )?,
            control_alignment: alignments(
                derivative,
                viscous,
                conservative_m384,
                control_cross,
            )?,
            base_cancellation: cancellation(derivative, viscous, conservative_m768, residual_m768)?,
            control_cancellation: cancellation(derivative, viscous, conservative_m384, residual_m384)?,
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

    fn orthogonal_sum(left: Self, right: Self) -> Result<Self, SolverError> {
        let add_cross = |a: [SignedNormChannels; 3], b: [SignedNormChannels; 3]| {
            std::array::from_fn(|i| SignedNormChannels {
                l2: a[i].l2 + b[i].l2,
                h1: a[i].h1 + b[i].h1,
                vorticity_l2: a[i].vorticity_l2 + b[i].vorticity_l2,
                divergence_l2: a[i].divergence_l2 + b[i].divergence_l2,
            })
        };
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

fn cancellation(
    a: Norms,
    b: Norms,
    c: Norms,
    r: Norms,
) -> Result<CancellationChannels, SolverError> {
    let ratio = |x: f64, y: f64, z: f64, result: f64| {
        let denominator = x + y + z;
        if denominator == 0.0 {
            None
        } else {
            Some(result / denominator)
        }
    };
    let values = [
        ratio(a.l2, b.l2, c.l2, r.l2),
        ratio(a.h1, b.h1, c.h1, r.h1),
        ratio(
            a.vorticity_l2,
            b.vorticity_l2,
            c.vorticity_l2,
            r.vorticity_l2,
        ),
        ratio(
            a.divergence_l2,
            b.divergence_l2,
            c.divergence_l2,
            r.divergence_l2,
        ),
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(SolverError::ArithmeticResolutionLimited);
    }
    Ok(CancellationChannels {
        l2: values[0],
        h1: values[1],
        vorticity_l2: values[2],
        divergence_l2: values[3],
    })
}

fn identity_error(
    a: Norms,
    b: Norms,
    c: Norms,
    r: Norms,
    cross: [SignedNormChannels; 3],
) -> Result<NormChannels, SolverError> {
    let relative = |x: f64, y: f64, z: f64, result: f64, cross: [f64; 3]| {
        let expected = x * x + y * y + z * z + cross.into_iter().sum::<f64>();
        let actual = result * result;
        let scale = actual.abs().max(expected.abs()).max(1.0);
        (actual - expected).abs() / scale
    };
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
