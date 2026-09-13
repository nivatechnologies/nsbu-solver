//! Optional allocation-free full-band ordered-Hessian screen.
//!
//! This remains separate from the established velocity/curl `NormOutput` contract. Fourier
//! coefficients are normalized physical averages, so `rms` is the Parseval volume-average
//! Frobenius norm and `l2` is the corresponding physical-domain integral norm.

use crate::model::{
    debug, AcceptanceOutput, AdmissionGuard, ComparisonKind, Evolution, Hashes, Manifest,
    ProfileBinding, ProfileBindingKind, Snapshot, TimeClockOutput,
};
use nsbu_solver::{
    domain::{validate_spectrum, Domain},
    spectral::modal,
    Complex64,
};
use serde::Serialize;

/// Norm of all 27 ordered `(velocity component, derivative axis, derivative axis)` entries.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct HessianNorms {
    pub l2: f64,
    pub rms: f64,
}

/// Separate Hessian-only comparison result; it does not alter the core norm output schema.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct HessianScreen {
    pub difference: HessianNorms,
    pub fine_absolute: HessianNorms,
    pub relative_to_fine: HessianRatios,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct HessianRatios {
    /// `None` records that the fine absolute denominator is exactly zero.
    pub l2: Option<f64>,
    /// `None` records that the fine absolute denominator is exactly zero.
    pub rms: Option<f64>,
}

/// Fully bound, separately serialized output for the optional Hessian screen.
#[derive(Debug, Serialize)]
pub(crate) struct HessianDiagnosticOutput<'a> {
    pub schema: &'static str,
    pub comparison_kind: &'static str,
    pub acceptance: AcceptanceOutput,
    pub normalization: &'static str,
    pub left_dimensions: [usize; 3],
    pub left_evolution: &'a Evolution,
    pub left_identity: &'a str,
    pub left_profile: Option<&'a ProfileBinding>,
    pub left_admission_guard: Option<&'a AdmissionGuard>,
    pub left_backend: &'a str,
    pub left_execution: &'a str,
    pub left_source_commit: &'a str,
    pub left_plan_sha256: &'a str,
    pub left_hashes: Hashes<'a>,
    pub right_dimensions: [usize; 3],
    pub right_evolution: &'a Evolution,
    pub right_identity: &'a str,
    pub right_profile: Option<&'a ProfileBinding>,
    pub right_admission_guard: Option<&'a AdmissionGuard>,
    pub right_backend: &'a str,
    pub right_execution: &'a str,
    pub right_source_commit: &'a str,
    pub right_plan_sha256: &'a str,
    pub right_hashes: Hashes<'a>,
    pub clock: TimeClockOutput,
    pub hessian: HessianScreen,
    pub admitted_bytes: usize,
}

pub(crate) fn diagnostic_output<'a>(
    left_manifest: &'a Manifest,
    left: &'a Snapshot,
    right_manifest: &'a Manifest,
    right: &'a Snapshot,
    admitted_bytes: usize,
) -> Result<HessianDiagnosticOutput<'a>, String> {
    validate_manifest_pair(left_manifest, right_manifest)?;
    let hessian = screen_ordered_hessian(
        left_manifest.domain()?,
        std::array::from_fn(|axis| left.coefficients[axis].as_slice()),
        right_manifest.domain()?,
        std::array::from_fn(|axis| right.coefficients[axis].as_slice()),
    )?;
    Ok(HessianDiagnosticOutput {
        schema: "p10-snapshot-ordered-hessian-diagnostic-output-v1",
        comparison_kind: "ORDERED_HESSIAN_DIAGNOSTIC",
        acceptance: AcceptanceOutput {
            status: "not_assessed",
            accepted_windows: 0,
        },
        normalization: "all-27-ordered-entries; rms=volume-average; l2=physical-domain-integral",
        left_dimensions: left_manifest.dimensions,
        left_evolution: &left_manifest.evolution,
        left_identity: &left_manifest.identity,
        left_profile: left_manifest.profile.as_ref(),
        left_admission_guard: left_manifest.admission_guard.as_ref(),
        left_backend: &left_manifest.backend,
        left_execution: &left_manifest.execution,
        left_source_commit: &left_manifest.source_commit,
        left_plan_sha256: &left_manifest.plan_sha256,
        left_hashes: snapshot_hashes(left),
        right_dimensions: right_manifest.dimensions,
        right_evolution: &right_manifest.evolution,
        right_identity: &right_manifest.identity,
        right_profile: right_manifest.profile.as_ref(),
        right_admission_guard: right_manifest.admission_guard.as_ref(),
        right_backend: &right_manifest.backend,
        right_execution: &right_manifest.execution,
        right_source_commit: &right_manifest.source_commit,
        right_plan_sha256: &right_manifest.plan_sha256,
        right_hashes: snapshot_hashes(right),
        clock: TimeClockOutput {
            elapsed: left.clock.elapsed,
            target: left.clock.target,
            left_epoch: left.clock.epoch,
            right_epoch: right.clock.epoch,
            left_accepted_steps: left.clock.accepted_steps,
            right_accepted_steps: right.clock.accepted_steps,
        },
        hessian,
        admitted_bytes,
    })
}

pub(crate) fn validate_manifest_pair(left: &Manifest, right: &Manifest) -> Result<(), String> {
    if left.comparison_kind != ComparisonKind::MatchedSpatial
        || right.comparison_kind != ComparisonKind::MatchedSpatial
        || left.elapsed != right.elapsed
        || left.target != right.target
        || left.epoch != right.epoch
        || left.accepted_steps != right.accepted_steps
    {
        return Err("ordered Hessian diagnostic requires matched spatial clocks".into());
    }
    validate_manifest_side(left)?;
    validate_manifest_side(right)
}

fn validate_manifest_side(manifest: &Manifest) -> Result<(), String> {
    let profile = manifest
        .profile
        .as_ref()
        .ok_or("ordered Hessian diagnostic requires an exact profile binding")?;
    let guard = manifest
        .admission_guard
        .as_ref()
        .ok_or("ordered Hessian diagnostic requires admission guard metadata")?;
    if !profile_matches_identity(&manifest.identity, profile)
        || manifest.epoch != manifest.accepted_steps
        || manifest.accepted_steps > guard.maximum_attempts
        || schedule_steps(&manifest.evolution)? != manifest.accepted_steps
    {
        return Err("ordered Hessian manifest binding mismatch".into());
    }
    Ok(())
}

fn profile_matches_identity(identity: &str, profile: &ProfileBinding) -> bool {
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
            .ok_or_else(|| "invalid ordered Hessian schedule".to_string())?;
        sum.checked_add(span / segment.step_ticks)
            .ok_or_else(|| "ordered Hessian schedule step count overflow".into())
    })
}

/// Measure a complete strict-band spectrum without allocating or transforming it.
pub(crate) fn measure_ordered_hessian(
    domain: Domain,
    values: [&[Complex64]; 3],
) -> Result<HessianNorms, String> {
    validate(domain, values)?;
    let mut squares = ScaledSquares::default();
    accumulate_absolute(domain, values, &mut squares)?;
    finish(domain, squares)
}

/// Compare the full fine band to a componentwise coarser spectrum without allocating.
pub(crate) fn screen_ordered_hessian(
    coarse_domain: Domain,
    coarse: [&[Complex64]; 3],
    fine_domain: Domain,
    fine: [&[Complex64]; 3],
) -> Result<HessianScreen, String> {
    validate_pair(coarse_domain, coarse, fine_domain, fine)?;
    let mut difference = ScaledSquares::default();
    accumulate_comparison(coarse_domain, coarse, fine_domain, fine, &mut difference)?;
    let difference = finish(fine_domain, difference)?;
    let fine_absolute = measure_ordered_hessian(fine_domain, fine)?;
    Ok(HessianScreen {
        difference,
        fine_absolute,
        relative_to_fine: HessianRatios {
            l2: ratio(difference.l2, fine_absolute.l2)?,
            rms: ratio(difference.rms, fine_absolute.rms)?,
        },
    })
}

fn validate(domain: Domain, values: [&[Complex64]; 3]) -> Result<(), String> {
    for component in values {
        validate_spectrum(domain.layout(), component, 1e-12).map_err(debug)?;
    }
    Ok(())
}

fn validate_pair(
    coarse_domain: Domain,
    coarse: [&[Complex64]; 3],
    fine_domain: Domain,
    fine: [&[Complex64]; 3],
) -> Result<(), String> {
    if coarse_domain.lengths() != fine_domain.lengths()
        || coarse_domain.viscosity() != fine_domain.viscosity()
        || coarse_domain
            .layout()
            .dimensions()
            .into_iter()
            .zip(fine_domain.layout().dimensions())
            .any(|(coarse, fine)| coarse > fine)
    {
        return Err("invalid Hessian comparison domains".into());
    }
    validate(coarse_domain, coarse)?;
    validate(fine_domain, fine)
}

fn accumulate_absolute(
    domain: Domain,
    values: [&[Complex64]; 3],
    squares: &mut ScaledSquares,
) -> Result<(), String> {
    let layout = domain.layout();
    for (index, _) in values[0].iter().enumerate() {
        let position = layout.position(index).map_err(debug)?;
        if layout.is_nyquist(position).map_err(debug)? {
            continue;
        }
        let wave =
            modal::wavevector(domain, layout.mode(position).map_err(debug)?).map_err(debug)?;
        push_mode(
            squares,
            std::array::from_fn(|axis| values[axis][index]),
            wave,
            layout.weight(position).map_err(debug)?,
        )?;
    }
    Ok(())
}

fn accumulate_comparison(
    coarse_domain: Domain,
    coarse: [&[Complex64]; 3],
    fine_domain: Domain,
    fine: [&[Complex64]; 3],
    difference: &mut ScaledSquares,
) -> Result<(), String> {
    let coarse_layout = coarse_domain.layout();
    let fine_layout = fine_domain.layout();
    for (index, _) in fine[0].iter().enumerate() {
        let position = fine_layout.position(index).map_err(debug)?;
        if fine_layout.is_nyquist(position).map_err(debug)? {
            continue;
        }
        let mode = fine_layout.mode(position).map_err(debug)?;
        let coarse_index = coarse_layout.locate(mode).ok().map(|(index, _)| index);
        let fine_mode: [Complex64; 3] = std::array::from_fn(|axis| fine[axis][index]);
        let delta = std::array::from_fn(|axis| {
            fine_mode[axis] - coarse_index.map_or(Complex64::new(0.0, 0.0), |i| coarse[axis][i])
        });
        let wave = modal::wavevector(fine_domain, mode).map_err(debug)?;
        let weight = fine_layout.weight(position).map_err(debug)?;
        push_mode(difference, delta, wave, weight)?;
    }
    Ok(())
}

fn push_mode(
    squares: &mut ScaledSquares,
    values: [Complex64; 3],
    wave: [f64; 3],
    weight: f64,
) -> Result<(), String> {
    // Sum over all nine ordered derivative pairs equals `(kx^2 + ky^2 + kz^2)^2`.
    let k_squared = wave.into_iter().map(|value| value * value).sum::<f64>();
    for value in values {
        squares.complex(value * k_squared, weight)?;
    }
    Ok(())
}

fn finish(domain: Domain, squares: ScaledSquares) -> Result<HessianNorms, String> {
    let rms = squares.norm()?;
    let volume_root = domain.lengths().into_iter().map(f64::sqrt).product::<f64>();
    let l2 = rms * volume_root;
    if !l2.is_finite() {
        return Err("Hessian norm arithmetic is nonfinite".into());
    }
    Ok(HessianNorms { l2, rms })
}

fn ratio(numerator: f64, denominator: f64) -> Result<Option<f64>, String> {
    if denominator == 0.0 {
        return Ok(None);
    }
    let result = numerator / denominator;
    if !result.is_finite() {
        return Err("Hessian ratio arithmetic is nonfinite".into());
    }
    Ok(Some(result))
}

fn snapshot_hashes(snapshot: &Snapshot) -> Hashes<'_> {
    Hashes {
        coefficient_sha256: &snapshot.coefficient_sha256,
        file_sha256: &snapshot.file_sha256,
    }
}

#[derive(Clone, Copy, Debug, Default)]
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
            return Err("Hessian norm arithmetic is nonfinite".into());
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
        let result = self.scale * self.sum.sqrt();
        if result.is_finite() {
            Ok(result)
        } else {
            Err("Hessian norm arithmetic is nonfinite".into())
        }
    }
}
