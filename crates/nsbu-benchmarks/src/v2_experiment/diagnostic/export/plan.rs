use super::DiagnosticExportError;
use crate::v2_experiment::{
    diagnostic::{DiagnosticBounds, DiagnosticPlan, DiagnosticSettings, StartupProfile},
    FamilySettings,
};
use nsbu_solver::domain::TickClock;

// Per-event upper inventory: fewer than 4,096 binary64 values, 1,024 integer
// counters, 512 complete clocks, 64 hashes, and 256 KiB of fixed keys/punctuation.
// Widths cover `.17e` including sign/exponent, quoted u128/usize decimal, and hex.
const FLOAT_FIELDS: usize = 4_096;
const COUNTER_FIELDS: usize = 1_024;
const CLOCK_FIELDS: usize = 512;
const HASH_FIELDS: usize = 64;
const FIXED_EVENT_BYTES: usize = 256 * 1024;
const FLOAT_WIDTH: usize = 32;
const COUNTER_WIDTH: usize = 41;
pub(super) const CLOCK_WIDTH: usize = 256;
const HASH_WIDTH: usize = 66;
const DOCUMENT_BYTES: usize = 64 * 1024;

/// Complete conservative serialization allowance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticExportBounds {
    /// Maximum JSON bytes accepted by admission.
    pub maximum_output_bytes: usize,
    /// Two complete visits: validation and emission.
    pub maximum_byte_visits: usize,
    /// Underlying writer calls, including one terminal failed call.
    pub maximum_write_calls: usize,
    /// Required fixed event count.
    pub events: usize,
}

/// Actual successful validation and emission work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticExportWork {
    /// Bytes visited by the validation pass.
    pub validation_bytes: usize,
    /// Bytes accepted by the caller writer.
    pub output_bytes: usize,
    /// Calls made to the validation sink.
    pub validation_calls: usize,
    /// Successful calls made to the caller writer.
    pub output_calls: usize,
    /// Complete events serialized.
    pub events: usize,
}

/// Owned identity and fixed-profile context for stable version 1 or explicit version 2.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticExportPlan {
    pub(super) version: usize,
    pub(super) accepted: [TickClock; 3],
    pub(super) manifest: [TickClock; 7],
    pub(super) residual: [TickClock; 4],
    pub(super) family: FamilySettings,
    pub(super) settings: DiagnosticSettings,
    pub(super) family_identity: [u8; 32],
    pub(super) probe_identity: [u8; 32],
    pub(super) coordinator: DiagnosticBounds,
    pub(super) branches: [BranchContext; 6],
    pub(super) probe_domains: [nsbu_solver::domain::Domain; 6],
    pub(super) pressure_source: nsbu_solver::domain::Domain,
    pub(super) pressure_layout: nsbu_solver::domain::Layout,
    pub(super) residual_force: nsbu_solver::domain::Layout,
    bounds: DiagnosticExportBounds,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BranchContext {
    pub grid: usize,
    pub step: u128,
    pub method: nsbu_solver::integrators::method::Method,
}

impl DiagnosticExportPlan {
    /// Admit the fixed startup profile without allocating report or output storage.
    pub fn new(
        profile: &StartupProfile,
        diagnostic: DiagnosticPlan<'_>,
        output_cap: usize,
    ) -> Result<Self, DiagnosticExportError> {
        Self::build(profile, diagnostic, output_cap, 1)
    }
    /// Admit explicit schema version 2 with reconstructed physical findings.
    pub fn new_v2(
        profile: &StartupProfile,
        diagnostic: DiagnosticPlan<'_>,
        output_cap: usize,
    ) -> Result<Self, DiagnosticExportError> {
        Self::build(profile, diagnostic, output_cap, 2)
    }
    fn build(
        profile: &StartupProfile,
        diagnostic: DiagnosticPlan<'_>,
        output_cap: usize,
        version: usize,
    ) -> Result<Self, DiagnosticExportError> {
        let accepted = *profile.accepted_times();
        let manifest = *profile.manifest();
        let residual = *profile.residual_times();
        if diagnostic.family_plan().times().as_slice() != accepted
            || diagnostic.probe_plan().tested_times().as_slice() != manifest
            || diagnostic.residual_times() != residual
        {
            return Err(DiagnosticExportError::InvalidReport);
        }
        let bounds = reservation(manifest.len(), output_cap)?;
        let pressure_source = diagnostic
            .family_plan()
            .branch_plan(2)
            .ok_or(DiagnosticExportError::InvalidReport)?
            .settings()
            .domain;
        let pressure_layout =
            nsbu_solver::diagnostics::conservative::ConservativeWorkspace::diagnostic_domain(
                pressure_source,
            )
            .map_err(|_| DiagnosticExportError::SizeOverflow)?
            .layout();
        let residual_force = diagnostic
            .family_plan()
            .settings()
            .force
            .double_grid()
            .map_err(|_| DiagnosticExportError::SizeOverflow)?
            .samples;
        Ok(Self {
            version,
            accepted,
            manifest,
            residual,
            family: diagnostic.family_plan().settings(),
            settings: diagnostic.diagnostic_settings(),
            family_identity: diagnostic.family_plan().identity(),
            probe_identity: diagnostic.probe_plan().identity(),
            coordinator: diagnostic.bounds(),
            branches: [
                branch(diagnostic, 0)?,
                branch(diagnostic, 1)?,
                branch(diagnostic, 2)?,
                branch(diagnostic, 3)?,
                branch(diagnostic, 4)?,
                branch(diagnostic, 5)?,
            ],
            probe_domains: [
                domain(diagnostic, 0)?,
                domain(diagnostic, 1)?,
                domain(diagnostic, 2)?,
                domain(diagnostic, 3)?,
                domain(diagnostic, 4)?,
                domain(diagnostic, 5)?,
            ],
            pressure_source,
            pressure_layout,
            residual_force,
            bounds,
        })
    }
    /// Selected JSON schema version, either 1 or 2.
    pub fn schema_version(self) -> usize {
        self.version
    }
    /// Complete output and work reservation.
    pub fn bounds(self) -> DiagnosticExportBounds {
        self.bounds
    }
    /// Required event count.
    pub fn event_count(self) -> usize {
        self.manifest.len()
    }
    /// Pressure construction layout derived from doubled finest velocity N.
    pub fn pressure_layout(self) -> nsbu_solver::domain::Layout {
        self.pressure_layout
    }
    /// Residual force-sampling layout derived independently from doubled force M.
    pub fn residual_force_layout(self) -> nsbu_solver::domain::Layout {
        self.residual_force
    }
}

fn domain(
    diagnostic: DiagnosticPlan<'_>,
    index: usize,
) -> Result<nsbu_solver::domain::Domain, DiagnosticExportError> {
    diagnostic
        .family_plan()
        .branch_plan(index)
        .ok_or(DiagnosticExportError::InvalidReport)
        .map(|plan| plan.resources().domain())
}

fn branch(
    diagnostic: DiagnosticPlan<'_>,
    index: usize,
) -> Result<BranchContext, DiagnosticExportError> {
    let settings = diagnostic
        .family_plan()
        .branch_plan(index)
        .ok_or(DiagnosticExportError::InvalidReport)?
        .settings();
    Ok(BranchContext {
        grid: settings.domain.layout().dimensions()[0],
        step: settings.configuration.limits.step_ticks,
        method: settings.configuration.method,
    })
}
fn reservation(events: usize, cap: usize) -> Result<DiagnosticExportBounds, DiagnosticExportError> {
    let widths = [
        (FLOAT_FIELDS, FLOAT_WIDTH),
        (COUNTER_FIELDS, COUNTER_WIDTH),
        (CLOCK_FIELDS, CLOCK_WIDTH),
        (HASH_FIELDS, HASH_WIDTH),
    ];
    let per_event = widths
        .into_iter()
        .try_fold(FIXED_EVENT_BYTES, |sum, (count, width)| {
            sum.checked_add(count.checked_mul(width)?)
        })
        .ok_or(DiagnosticExportError::SizeOverflow)?;
    let maximum_output_bytes = events
        .checked_mul(per_event)
        .and_then(|n| n.checked_add(DOCUMENT_BYTES))
        .ok_or(DiagnosticExportError::SizeOverflow)?;
    let maximum_byte_visits = maximum_output_bytes
        .checked_mul(2)
        .ok_or(DiagnosticExportError::SizeOverflow)?;
    let maximum_write_calls = maximum_byte_visits
        .checked_add(1)
        .ok_or(DiagnosticExportError::SizeOverflow)?;
    if maximum_output_bytes > cap {
        return Err(DiagnosticExportError::ResourceLimit);
    }
    Ok(DiagnosticExportBounds {
        maximum_output_bytes,
        maximum_byte_visits,
        maximum_write_calls,
        events,
    })
}
