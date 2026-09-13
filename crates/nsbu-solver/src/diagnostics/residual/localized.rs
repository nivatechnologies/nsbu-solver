use super::{accumulation, ResidualPlan};
use crate::{domain::validate_spectrum, spectral::modal, Complex64, SolverError};
use accumulation::{BandSums, Terms};
use sha2::{Digest, Sha256};

pub use accumulation::{CancellationChannels, NormChannels, ResidualBandLocalization};

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

#[derive(Clone, Copy)]
struct Inputs<'a> {
    velocity: [&'a [Complex64]; 3],
    derivative: [&'a [Complex64]; 3],
    conservative_m768: [&'a [Complex64]; 3],
    force_m768: [&'a [Complex64]; 3],
    force_m384: [&'a [Complex64]; 3],
}

impl Inputs<'_> {
    fn validate(self, plan: ResidualPlan) -> Result<(), SolverError> {
        for values in self
            .velocity
            .into_iter()
            .chain(self.derivative)
            .chain(self.force_m384)
        {
            validate_spectrum(plan.source.layout(), values, 1e-12)?;
        }
        for values in self.conservative_m768.into_iter().chain(self.force_m768) {
            validate_spectrum(plan.diagnostic.layout(), values, 1e-12)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct ComponentHashes {
    delta: [Sha256; 3],
    control: [Sha256; 3],
}

impl ComponentHashes {
    fn push_zero(&mut self) {
        for hash in self.delta.iter_mut().chain(&mut self.control) {
            hash.update(0.0_f64.to_bits().to_le_bytes());
            hash.update(0.0_f64.to_bits().to_le_bytes());
        }
    }

    fn push(&mut self, delta: [Complex64; 3], control: [Complex64; 3]) {
        for axis in 0..3 {
            for (hash, value) in [
                (&mut self.delta[axis], delta[axis]),
                (&mut self.control[axis], control[axis]),
            ] {
                hash.update(value.re.to_bits().to_le_bytes());
                hash.update(value.im.to_bits().to_le_bytes());
            }
        }
    }

    fn finish(self) -> ([[u8; 32]; 3], [[u8; 32]; 3]) {
        (
            self.delta.map(|hash| hash.finalize().into()),
            self.control.map(|hash| hash.finalize().into()),
        )
    }
}

struct Traversal<'a, 'output> {
    plan: ResidualPlan,
    inputs: Inputs<'a>,
    output: [&'output mut [Complex64]; 3],
    retained: BandSums,
    shell: BandSums,
    retained_modes: usize,
    new_shell_modes: usize,
    excluded_nyquist_slots: usize,
    hashes: ComponentHashes,
}

impl<'a, 'output> Traversal<'a, 'output> {
    fn new(plan: ResidualPlan, inputs: Inputs<'a>, output: [&'output mut [Complex64]; 3]) -> Self {
        Self {
            plan,
            inputs,
            output,
            retained: BandSums::default(),
            shell: BandSums::default(),
            retained_modes: 0,
            new_shell_modes: 0,
            excluded_nyquist_slots: 0,
            hashes: ComponentHashes::default(),
        }
    }

    fn run(mut self) -> Result<ResidualLocalization, SolverError> {
        for index in 0..self.plan.diagnostic.layout().half_len() {
            self.visit(index)?;
        }
        self.finish()
    }

    fn visit(&mut self, index: usize) -> Result<(), SolverError> {
        let layout = self.plan.diagnostic.layout();
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            self.record_nyquist(index);
            return Ok(());
        }
        let mode = layout.mode(position)?;
        let k = modal::wavevector(self.plan.diagnostic, mode)?;
        let located = self.plan.source.layout().locate(mode).ok().map(|(i, _)| i);
        let terms = self.assemble_terms(index, k, located)?;
        self.record_mode(index, position, k, located, terms)
    }

    fn assemble_terms(
        &self,
        index: usize,
        k: [f64; 3],
        located: Option<usize>,
    ) -> Result<Terms, SolverError> {
        let zero = Complex64::new(0.0, 0.0);
        let d =
            std::array::from_fn(|axis| located.map_or(zero, |i| self.inputs.derivative[axis][i]));
        let mut v = [zero; 3];
        let mut r = std::array::from_fn(|axis| self.inputs.conservative_m768[axis][index]);
        for axis in 0..3 {
            r[axis] += d[axis];
            if let Some(source_index) = located {
                for wave in k {
                    let contribution = self.plan.source.viscosity()
                        * wave
                        * (wave * self.inputs.velocity[axis][source_index]);
                    v[axis] += contribution;
                    r[axis] += contribution;
                }
            }
        }
        let raw_force_delta = std::array::from_fn(|axis| {
            self.inputs.force_m768[axis][index]
                - located.map_or(zero, |i| self.inputs.force_m384[axis][i])
        });
        let projected_force_delta = modal::project(k, raw_force_delta)?;
        let conservative_control = std::array::from_fn(|axis| {
            self.inputs.conservative_m768[axis][index] + projected_force_delta[axis]
        });
        let residual_control = std::array::from_fn(|axis| r[axis] + projected_force_delta[axis]);
        Ok([
            d,
            v,
            std::array::from_fn(|axis| self.inputs.conservative_m768[axis][index]),
            r,
            projected_force_delta,
            conservative_control,
            residual_control,
        ])
    }

    fn record_nyquist(&mut self, index: usize) {
        let zero = Complex64::new(0.0, 0.0);
        self.write(index, [zero; 3]);
        self.hashes.push_zero();
        self.excluded_nyquist_slots += 1;
    }

    fn record_mode(
        &mut self,
        index: usize,
        position: [usize; 3],
        k: [f64; 3],
        located: Option<usize>,
        terms: Terms,
    ) -> Result<(), SolverError> {
        self.hashes.push(terms[4], terms[6]);
        self.write(index, terms[3]);
        if located.is_some() {
            self.retained_modes += 1;
            self.retained
                .push(k, terms, self.plan.diagnostic.layout().weight(position)?)
        } else {
            self.record_shell(position, k, terms)
        }
    }

    fn record_shell(
        &mut self,
        position: [usize; 3],
        k: [f64; 3],
        terms: Terms,
    ) -> Result<(), SolverError> {
        self.new_shell_modes += 1;
        let zero = Complex64::new(0.0, 0.0);
        if terms[3] != terms[2] || terms[0] != [zero; 3] || terms[1] != [zero; 3] {
            return Err(SolverError::InvalidSpectrum);
        }
        self.shell
            .push(k, terms, self.plan.diagnostic.layout().weight(position)?)
    }

    fn write(&mut self, index: usize, value: [Complex64; 3]) {
        self.output[0][index] = value[0];
        self.output[1][index] = value[1];
        self.output[2][index] = value[2];
    }

    fn finish(self) -> Result<ResidualLocalization, SolverError> {
        let retained = self.retained.finish()?;
        let shell = self.shell.finish()?;
        let full = ResidualBandLocalization::orthogonal_sum(retained, shell)?;
        let (delta_hashes, control_hashes) = self.hashes.finish();
        Ok(ResidualLocalization {
            retained_strict_n384: retained,
            new_shell_n768: shell,
            full_n768: full,
            retained_modes: self.retained_modes,
            new_shell_modes: self.new_shell_modes,
            excluded_nyquist_slots: self.excluded_nyquist_slots,
            projected_force_delta_component_sha256: delta_hashes,
            residual_m384_component_sha256: control_hashes,
        })
    }
}

impl ResidualPlan {
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
        let inputs = Inputs {
            velocity,
            derivative,
            conservative_m768,
            force_m768,
            force_m384,
        };
        inputs.validate(self)?;
        if output_m768
            .iter()
            .any(|values| values.len() != self.diagnostic.layout().half_len())
        {
            return Err(SolverError::InvalidPayload);
        }
        Traversal::new(self, inputs, output_m768).run()
    }
}
