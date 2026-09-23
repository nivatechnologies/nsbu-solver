use nsbu_solver::{domain::Domain, Complex64};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const INPUT_SCHEMA: &str = "p10-n256-m512-pair-input-v1";
pub(crate) const LINEAGE_SCHEMA: &str = "p10-n256-m512-lineage-intake-v1";

/// The exact metadata-only rest record the `n384-prep` capture writer publishes
/// via `step_artifact::publish_rest`: a small JSON status file, never a binary
/// state snapshot. `state_payload` is always false and there is no clock word.
pub(crate) const REST_SCHEMA: &str = "p10-avx-n384-rest-v1";
pub(crate) const REST_OBSERVATION_STATUS: &str = "RestExact";
pub(crate) const REST_BALANCE: &str = "REST";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestArtifact {
    pub schema: String,
    pub identity: String,
    pub clock: u128,
    pub state_payload: bool,
    pub observation_status: String,
    pub balance: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub schema: String,
    pub comparison_kind: ComparisonKind,
    pub snapshot: PathBuf,
    pub plan: PathBuf,
    pub identity: String,
    pub source_commit: String,
    pub plan_sha256: String,
    pub coefficient_sha256: String,
    pub file_sha256: String,
    pub backend: String,
    pub execution: String,
    pub dimensions: [usize; 3],
    pub evolution: Evolution,
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
    pub profile: ProfileBinding,
    pub admission_guard: AdmissionGuard,
    pub lineage: Option<LineageBinding>,
}

impl Manifest {
    pub(crate) fn domain(&self) -> Result<Domain, String> {
        Domain::new(
            self.dimensions,
            self.evolution.lengths,
            self.evolution.viscosity,
        )
        .map_err(debug)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LineageBinding {
    pub intake: PathBuf,
    pub intake_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) enum ComparisonKind {
    #[serde(rename = "N256_M512_PAIR_ENDPOINT_DIAGNOSTIC")]
    N256M512PairEndpointDiagnostic,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdmissionGuard {
    pub advective_limit: f64,
    pub maximum_attempts: u128,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileBinding {
    pub kind: ProfileBindingKind,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) enum ProfileBindingKind {
    #[serde(rename = "identity-profile-field")]
    IdentityProfileField,
}

impl ProfileBinding {
    pub(crate) fn matches_identity(&self, identity: &str) -> bool {
        self.kind == ProfileBindingKind::IdentityProfileField
            && !self.value.is_empty()
            && identity
                .split(';')
                .filter(|field| field.starts_with("profile="))
                .eq([format!("profile={}", self.value).as_str()])
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Evolution {
    pub case_sha256: String,
    pub quantum_exponent: i32,
    pub clock_target: u128,
    pub comparison_endpoint: u128,
    pub lengths: [f64; 3],
    pub viscosity: f64,
    pub method: String,
    pub integration_force_dimensions: [usize; 3],
    pub schedule: Vec<ScheduleSegment>,
    pub absolute_tolerances: [f64; 2],
    pub relative_tolerances: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScheduleSegment {
    pub from_inclusive: u128,
    pub until_exclusive: u128,
    pub step_ticks: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClockHeader {
    pub elapsed: u128,
    pub target: u128,
    pub epoch: u128,
    pub accepted_steps: u128,
}

impl ClockHeader {
    pub(crate) fn from_manifest(manifest: &Manifest) -> Self {
        Self {
            elapsed: manifest.elapsed,
            target: manifest.target,
            epoch: manifest.epoch,
            accepted_steps: manifest.accepted_steps,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub coefficients: [Vec<Complex64>; 3],
    pub coefficient_sha256: String,
    pub file_sha256: String,
    pub clock: ClockHeader,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LineageIntake {
    pub schema: String,
    pub host: String,
    pub profile: String,
    pub source_commit: String,
    pub plan_sha256: String,
    pub binary_sha256: String,
    pub from_rest: bool,
    pub rest: RestRecord,
    pub states: Vec<StateRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestRecord {
    pub path: PathBuf,
    pub file_sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateRecord {
    pub clock: u128,
    pub path: PathBuf,
    pub coefficient_sha256: String,
    pub file_sha256: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Hashes<'a> {
    pub coefficient_sha256: &'a str,
    pub file_sha256: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClockOutput {
    pub elapsed: u128,
    pub target: u128,
    pub coarse_epoch: u128,
    pub fine_epoch: u128,
    pub coarse_accepted_steps: u128,
    pub fine_accepted_steps: u128,
}

#[derive(Debug, Serialize)]
pub(crate) struct AcceptanceOutput {
    pub status: &'static str,
    pub accepted_windows: u8,
}

#[derive(Debug, Serialize)]
pub(crate) struct InterpretationOutput {
    pub kind: &'static str,
    pub endpoint_clock: u128,
    pub endpoint_physical_time: &'static str,
    pub quantum: &'static str,
    pub endpoint_only: bool,
    pub converged_pde_window: bool,
    pub window_qualification: &'static str,
    pub refinement_model_note: &'static str,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_field_names)]
pub(crate) struct DiagnosticOutput<'a> {
    pub schema: &'static str,
    pub comparison_kind: &'static str,
    pub acceptance: AcceptanceOutput,
    pub interpretation: InterpretationOutput,
    pub lineage_status: &'static str,
    pub coarse_lineage_states: usize,
    pub left_evolution: &'a Evolution,
    pub right_evolution: &'a Evolution,
    pub left_identity: &'a str,
    pub left_profile: &'a ProfileBinding,
    pub left_source_commit: &'a str,
    pub left_plan_sha256: &'a str,
    pub left_lineage_intake_sha256: String,
    pub right_identity: &'a str,
    pub right_profile: &'a ProfileBinding,
    pub right_source_commit: &'a str,
    pub right_plan_sha256: &'a str,
    pub left_hashes: Hashes<'a>,
    pub right_hashes: Hashes<'a>,
    pub clock: ClockOutput,
    pub full: NormOutput,
    pub common: NormOutput,
    pub newly_resolved: NormOutput,
    pub fine_absolute: NormOutput,
    pub mean_error: [f64; 3],
    pub admitted_bytes: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct NormOutput {
    pub l2: f64,
    pub h1: f64,
    pub vorticity_l2: f64,
    pub divergence_l2: f64,
}

impl From<nsbu_solver::diagnostics::norms::Norms> for NormOutput {
    fn from(value: nsbu_solver::diagnostics::norms::Norms) -> Self {
        Self {
            l2: value.l2,
            h1: value.h1,
            vorticity_l2: value.vorticity_l2,
            divergence_l2: value.divergence_l2,
        }
    }
}

pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
