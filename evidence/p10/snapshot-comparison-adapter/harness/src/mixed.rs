//! Closed three-state force/space sensitivity diagnostic.

use crate::model::{
    debug, AcceptanceOutput, AdmissionGuard, ComparisonKind, Evolution, Hashes, Manifest,
    ProfileBinding, ProfileBindingKind, Snapshot,
};
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

#[derive(Debug, Serialize)]
pub(crate) struct SideOutput<'a> {
    pub role: &'static str,
    pub dimensions: [usize; 3],
    pub evolution: &'a Evolution,
    pub identity: &'a str,
    pub profile: &'a ProfileBinding,
    pub admission_guard: &'a AdmissionGuard,
    pub backend: &'a str,
    pub execution: &'a str,
    pub source_commit: &'a str,
    pub plan_sha256: &'a str,
    pub hashes: Hashes<'a>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct MixedClock {
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct MixedDiagnosticOutput<'a> {
    pub schema: &'static str,
    pub comparison_kind: &'static str,
    pub acceptance: AcceptanceOutput,
    pub coarse: SideOutput<'a>,
    pub baseline: SideOutput<'a>,
    pub force: SideOutput<'a>,
    pub clock: MixedClock,
    pub metrics: MixedMetrics,
    pub admitted_bytes: usize,
}

pub(crate) fn diagnostic_output<'a>(
    coarse_manifest: &'a Manifest,
    coarse: &'a Snapshot,
    baseline_manifest: &'a Manifest,
    baseline: &'a Snapshot,
    force_manifest: &'a Manifest,
    force: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<MixedDiagnosticOutput<'a>, String> {
    validate_manifests(coarse_manifest, baseline_manifest, force_manifest)?;
    if coarse.clock != baseline.clock || baseline.clock != force.clock {
        return Err("mixed diagnostic snapshot clock mismatch".into());
    }
    let metrics = calculate(
        coarse_manifest.domain()?,
        arrays(coarse),
        baseline_manifest.domain()?,
        arrays(baseline),
        force_manifest.domain()?,
        arrays(force),
    )?;
    Ok(MixedDiagnosticOutput {
        schema: "p10-snapshot-mixed-force-space-diagnostic-output-v1",
        comparison_kind: "MIXED_FORCE_SPACE_DIAGNOSTIC",
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        coarse: side("coarse_u256_m384", coarse_manifest, coarse),
        baseline: side("baseline_u384_m384", baseline_manifest, baseline),
        force: side("force_u384_m512", force_manifest, force),
        clock: MixedClock {
            elapsed: coarse.clock.elapsed,
            target: coarse.clock.target,
            epoch: coarse.clock.epoch,
            accepted_steps: coarse.clock.accepted_steps,
        },
        metrics,
        admitted_bytes,
    })
}

pub(crate) fn validate_manifests(
    coarse: &Manifest,
    baseline: &Manifest,
    force: &Manifest,
) -> Result<(), String> {
    if [
        coarse.comparison_kind,
        baseline.comparison_kind,
        force.comparison_kind,
    ] != [ComparisonKind::MixedForceSpaceDiagnostic; 3]
        || coarse.dimensions != [256; 3]
        || baseline.dimensions != [384; 3]
        || force.dimensions != [384; 3]
        || coarse.evolution.integration_force_dimensions != [384; 3]
        || baseline.evolution.integration_force_dimensions != [384; 3]
        || force.evolution.integration_force_dimensions != [512; 3]
        || coarse.evolution != baseline.evolution
        || !same_except_force(&baseline.evolution, &force.evolution)
    {
        return Err("mixed force/space diagnostic contract mismatch".into());
    }
    if manifest_clock(coarse) != manifest_clock(baseline)
        || manifest_clock(baseline) != manifest_clock(force)
    {
        return Err("mixed force/space diagnostic clock mismatch".into());
    }
    validate_side(coarse)?;
    validate_side(baseline)?;
    validate_side(force)
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

fn validate_side(manifest: &Manifest) -> Result<(), String> {
    let profile = manifest.profile.as_ref().ok_or("missing exact profile")?;
    let guard = manifest
        .admission_guard
        .as_ref()
        .ok_or("missing admission guard metadata")?;
    if manifest.arithmetic_control.is_some()
        || !profile_matches(&manifest.identity, profile)
        || manifest.epoch != manifest.accepted_steps
        || manifest.accepted_steps > guard.maximum_attempts
        || schedule_steps(&manifest.evolution)? != manifest.accepted_steps
    {
        return Err("mixed diagnostic manifest binding mismatch".into());
    }
    Ok(())
}

fn same_except_force(left: &Evolution, right: &Evolution) -> bool {
    left.case_sha256 == right.case_sha256
        && left.quantum_exponent == right.quantum_exponent
        && left.clock_target == right.clock_target
        && left.comparison_endpoint == right.comparison_endpoint
        && bits(left.lengths, right.lengths)
        && left.viscosity.to_bits() == right.viscosity.to_bits()
        && left.method == right.method
        && left.schedule == right.schedule
        && bits(left.absolute_tolerances, right.absolute_tolerances)
        && bits(left.relative_tolerances, right.relative_tolerances)
}

fn bits<const N: usize>(left: [f64; N], right: [f64; N]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn profile_matches(identity: &str, profile: &ProfileBinding) -> bool {
    match profile.kind {
        ProfileBindingKind::LegacyFullIdentity => profile.value == identity,
        ProfileBindingKind::IdentityProfileField => {
            !profile.value.is_empty()
                && identity
                    .split(';')
                    .find_map(|field| field.strip_prefix("profile="))
                    == Some(profile.value.as_str())
        }
    }
}

fn schedule_steps(evolution: &Evolution) -> Result<u128, String> {
    evolution.schedule.iter().try_fold(0_u128, |sum, segment| {
        let span = segment
            .until_exclusive
            .checked_sub(segment.from_inclusive)
            .filter(|_| segment.step_ticks != 0)
            .ok_or_else(|| "invalid mixed diagnostic schedule".to_string())?;
        sum.checked_add(span / segment.step_ticks)
            .ok_or_else(|| "mixed diagnostic schedule step count overflow".into())
    })
}

fn manifest_clock(manifest: &Manifest) -> [u128; 4] {
    [
        manifest.elapsed,
        manifest.target,
        manifest.epoch,
        manifest.accepted_steps,
    ]
}

fn arrays(snapshot: &Snapshot) -> [&[Complex64]; 3] {
    std::array::from_fn(|axis| snapshot.coefficients[axis].as_slice())
}

fn side<'a>(role: &'static str, manifest: &'a Manifest, snapshot: &'a Snapshot) -> SideOutput<'a> {
    SideOutput {
        role,
        dimensions: manifest.dimensions,
        evolution: &manifest.evolution,
        identity: &manifest.identity,
        profile: manifest.profile.as_ref().unwrap(),
        admission_guard: manifest.admission_guard.as_ref().unwrap(),
        backend: &manifest.backend,
        execution: &manifest.execution,
        source_commit: &manifest.source_commit,
        plan_sha256: &manifest.plan_sha256,
        hashes: Hashes {
            coefficient_sha256: &snapshot.coefficient_sha256,
            file_sha256: &snapshot.file_sha256,
        },
    }
}

#[derive(Default)]
struct MixedSums {
    common: BandSums,
    newly: BandSums,
}

impl MixedSums {
    fn push(
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

    fn finish(self) -> Result<MixedMetrics, String> {
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

fn curl(k: [f64; 3], u: [Complex64; 3]) -> [Complex64; 3] {
    [
        k[1] * u[2] - k[2] * u[1],
        k[2] * u[0] - k[0] * u[2],
        k[0] * u[1] - k[1] * u[0],
    ]
}

fn real_inner(left: Complex64, right: Complex64) -> f64 {
    left.re * right.re + left.im * right.im
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

#[derive(Clone, Copy, Default)]
struct Compensated {
    sum: f64,
    correction: f64,
}

impl Compensated {
    fn push(&mut self, value: f64) -> Result<(), String> {
        let value = finite(value)?;
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = finite(next)?;
        Ok(())
    }

    fn finish(self) -> Result<f64, String> {
        finite(self.sum)
    }
}

#[derive(Clone, Copy, Default)]
struct ScaledSquares {
    scale: f64,
    sum: f64,
}

impl ScaledSquares {
    fn complex(&mut self, value: Complex64, weight: f64) -> Result<(), String> {
        self.push(value.re, weight)?;
        self.push(value.im, weight)
    }

    fn push(&mut self, value: f64, weight: f64) -> Result<(), String> {
        let value = value.abs();
        if !value.is_finite() || !weight.is_finite() || weight < 0.0 {
            return Err("mixed diagnostic norm arithmetic is nonfinite".into());
        }
        if value == 0.0 || weight == 0.0 {
            return Ok(());
        }
        let scale = self.scale.max(value);
        self.sum = self.sum * (self.scale / scale).powi(2) + weight * (value / scale).powi(2);
        self.scale = scale;
        Ok(())
    }

    fn norm(self) -> Result<f64, String> {
        finite(self.scale * self.sum.sqrt())
    }
}

fn finite(value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("mixed diagnostic arithmetic is nonfinite".into())
    }
}
