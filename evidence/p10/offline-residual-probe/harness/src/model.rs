use nsbu_solver::{
    diagnostics::{
        norms::{Norms, SignedNormChannels},
        residual::{
            CancellationChannels, NormChannels, ResidualBandLocalization, ResidualLocalization,
        },
    },
    domain::Domain,
    Complex64,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const SOURCE: &str = "aed49b7d7874a0a720dee88b65ba180c7286fa65";
pub(crate) const W3_SOURCE: &str = "f13c29c9ae91d0b8cf7a790132deb9bd076911c0";
pub(crate) const CASE_SHA256: &str =
    "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";
pub(crate) const PLAN_SHA256: &str =
    "2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a";
pub(crate) const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
pub(crate) const M512_SOURCE: &str = "326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72";
pub(crate) const M512_PLAN_SHA256: &str =
    "2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84";
pub(crate) const M512_PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
pub(crate) const M512_IDENTITY: &str = concat!(
    "source=326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72;",
    "case=e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e;",
    "profile=n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c;",
    "backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;",
    "provider=parallel-reduced-v2-force-w3-attempt-cache;",
    "rhs_w3=layout576-width3-bidirectional-add9200779136;",
    "force_w3=layout512-width3-forward-add4318334720;rhs_timer=harness-timed-rhs-v1;",
    "clock=std-time-Instant;scope=evaluate-inclusive;overhead=included;retained=384;",
    "force_samples=512;observer_force_samples=768;observer_conservative=768;",
    "sampling_workers=32;rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;",
    "schedule=h64-clocks0-through2048-then-h128-through4096;endpoint=4096;",
    "advective_limit=3.3;execution_cap=206158430208;artifact_cap=137438953472;",
    "schema=p10-avx-n384-every-step-v1;attempt_schema=p10-avx-scheduled-attempt-v3;",
    "resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;",
    "external_stop=pgid-watchdog-v2-starttime-cmdline-deadline"
);
pub(crate) const PROBE: u128 = 1112;
pub(crate) const SUPPORTS: [[u128; 3]; 3] =
    [[896, 1152, 1408], [1024, 1152, 1280], [1088, 1152, 1216]];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProbePlan {
    pub schema: String,
    pub status: String,
    pub source_commit: String,
    pub w3_source_commit: String,
    pub case_sha256: String,
    pub frozen_plan: PathBuf,
    pub frozen_plan_sha256: String,
    pub profile: String,
    pub snapshot_identity: String,
    pub dimensions: [usize; 3],
    pub lengths: [f64; 3],
    pub viscosity: f64,
    pub quantum_exponent: i32,
    pub clock_target: u128,
    pub method: String,
    pub integration_force_dimensions: [usize; 3],
    pub integration_force_workers: usize,
    pub residual_force_dimensions: [usize; 3],
    pub residual_force_workers: usize,
    pub advective_limit: f64,
    pub probe_clock: u128,
    pub supports: [[u128; 3]; 3],
    pub nodes: Vec<NodeBinding>,
    pub claims: Claims,
}

impl ProbePlan {
    pub(crate) fn domain(&self) -> Result<Domain, String> {
        Domain::new(self.dimensions, self.lengths, self.viscosity).map_err(debug)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeBinding {
    pub clock: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
    pub snapshot: PathBuf,
    pub coefficient_sha256: String,
    pub file_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Claims {
    pub runtime_owner_imported: bool,
    pub accepted_interpolation: bool,
    pub acceptance_windows: usize,
    pub arithmetic: String,
    pub purpose: String,
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub coefficients: [Vec<Complex64>; 3],
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct NormOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<Norms> for NormOutput {
    fn from(value: Norms) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ReservationOutput {
    pub schema: &'static str,
    pub source_state_bytes: usize,
    pub diagnostic_field_bytes: usize,
    pub fft_catalog_bytes: usize,
    pub integration_rhs_bytes: usize,
    pub integration_force_bytes: usize,
    pub residual_force_bytes: usize,
    pub conservative_workspace_bytes: usize,
    pub reconstruction_peak_bytes: usize,
    pub residual_peak_bytes: usize,
    pub admitted_peak_bytes: usize,
    pub cap_bytes: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct NodeOutput {
    pub clock: u128,
    pub state_file_sha256: String,
    pub coefficient_sha256: String,
    pub physical_derivative_sha256: String,
    pub rhs_calls: usize,
    pub rhs_work_units: usize,
    pub rhs_scalar_transforms: usize,
    pub force_cache_hits: usize,
    pub force_cache_misses: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct SignedNormOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<SignedNormChannels> for SignedNormOutput {
    fn from(value: SignedNormChannels) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct ChannelOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<NormChannels> for ChannelOutput {
    fn from(value: NormChannels) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct CancellationOutput {
    pub l2: Option<f64>,
    pub h1: Option<f64>,
    pub vorticity_l2: Option<f64>,
    pub divergence_l2: Option<f64>,
}

impl From<CancellationChannels> for CancellationOutput {
    fn from(value: CancellationChannels) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct BandLocalizationOutput {
    pub derivative: NormOutput,
    pub viscous: NormOutput,
    pub conservative_m768: NormOutput,
    pub residual_m768: NormOutput,
    pub projected_force_delta: NormOutput,
    pub conservative_m384: NormOutput,
    pub residual_m384: NormOutput,
    pub base_cross: [SignedNormOutput; 3],
    pub control_cross: [SignedNormOutput; 3],
    pub base_alignment: [CancellationOutput; 3],
    pub control_alignment: [CancellationOutput; 3],
    pub base_cancellation: CancellationOutput,
    pub control_cancellation: CancellationOutput,
    pub base_identity_relative_error: ChannelOutput,
    pub control_identity_relative_error: ChannelOutput,
}

impl From<ResidualBandLocalization> for BandLocalizationOutput {
    fn from(value: ResidualBandLocalization) -> Self {
        Self {
            derivative: value.derivative.into(),
            viscous: value.viscous.into(),
            conservative_m768: value.conservative_m768.into(),
            residual_m768: value.residual_m768.into(),
            projected_force_delta: value.projected_force_delta.into(),
            conservative_m384: value.conservative_m384.into(),
            residual_m384: value.residual_m384.into(),
            base_cross: value.base_cross.map(Into::into),
            control_cross: value.control_cross.map(Into::into),
            base_alignment: value.base_alignment.map(Into::into),
            control_alignment: value.control_alignment.map(Into::into),
            base_cancellation: value.base_cancellation.into(),
            control_cancellation: value.control_cancellation.into(),
            base_identity_relative_error: value.base_identity_relative_error.into(),
            control_identity_relative_error: value.control_identity_relative_error.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LocalizationOutput {
    pub retained_strict_n384: BandLocalizationOutput,
    pub new_shell_n768: BandLocalizationOutput,
    pub full_n768: BandLocalizationOutput,
    pub retained_modes: usize,
    pub new_shell_modes: usize,
    pub excluded_nyquist_slots: usize,
    pub projected_force_delta_component_sha256: [String; 3],
    pub residual_m384_component_sha256: [String; 3],
}

impl From<ResidualLocalization> for LocalizationOutput {
    fn from(value: ResidualLocalization) -> Self {
        Self {
            retained_strict_n384: value.retained_strict_n384.into(),
            new_shell_n768: value.new_shell_n768.into(),
            full_n768: value.full_n768.into(),
            retained_modes: value.retained_modes,
            new_shell_modes: value.new_shell_modes,
            excluded_nyquist_slots: value.excluded_nyquist_slots,
            projected_force_delta_component_sha256: value
                .projected_force_delta_component_sha256
                .map(hex),
            residual_m384_component_sha256: value.residual_m384_component_sha256.map(hex),
        }
    }
}

fn hex(bytes: [u8; 32]) -> String {
    bytes
        .into_iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Serialize)]
pub(crate) struct LocalizationRunOutput {
    pub schema: &'static str,
    pub status: &'static str,
    pub source_commit: &'static str,
    pub w3_source_commit: &'static str,
    pub case_sha256: &'static str,
    pub frozen_plan_sha256: &'static str,
    pub profile: &'static str,
    pub arithmetic: &'static str,
    pub integration_force: &'static str,
    pub base_residual_force: &'static str,
    pub discrete_retained_force_control: &'static str,
    pub retained_grid: [usize; 3],
    pub diagnostic_grid: [usize; 3],
    pub probe_clock: u128,
    pub support: [u128; 3],
    pub reservations: ReservationOutput,
    pub nodes: Vec<NodeOutput>,
    pub reconstructed_value_sha256: String,
    pub reconstructed_derivative_sha256: String,
    pub base_residual_sha256: String,
    pub base_force_work_units: usize,
    pub base_force_scalar_transforms: usize,
    pub control_force_work_units: usize,
    pub control_force_scalar_transforms: usize,
    pub localization: LocalizationOutput,
    pub identity_closure: IdentityClosureOutput,
    pub runtime_owner_imported: bool,
    pub accepted_interpolation: bool,
    pub acceptance_windows: usize,
    pub qualification: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct IdentityClosureOutput {
    pub residual_relative_tolerance: f64,
    pub residual_relative_maximum: f64,
    pub residual_relative_passed: bool,
    pub term_scaled_tolerance: f64,
    pub term_scaled_maximum: f64,
    pub term_scaled_passed: bool,
    pub term_scaled_bound_basis: &'static str,
}

pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
