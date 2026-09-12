//! Independent JSON decoding, complete-value, refusal, and writer-failure controls.
use nsbu_benchmarks::v2_experiment::diagnostic::{
    export::{write_json, DiagnosticExportError, DiagnosticExportPlan},
    DiagnosticDriver, StartupProfile,
};
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        diagnostic::{DiagnosticPlan, DiagnosticSettings},
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
mod v2_diagnostic_export_oracle;
use serde_json::Value;
use std::io::{self, Write};

const CAP: usize = 256 * 1024 * 1024;

#[test]
fn diagnostic_export_rejects_nonprofile_accepted_and_probe_schedules() {
    let profile = StartupProfile::new().unwrap();
    let standard = profile.plan(CAP).unwrap();
    let accepted = [profile.accepted_times()[0], profile.accepted_times()[2]];
    let manifest = [
        profile.manifest()[0],
        profile.manifest()[1],
        profile.manifest()[2],
        profile.manifest()[4],
        profile.manifest()[5],
        profile.manifest()[6],
    ];
    let family = FamilyPlan::new(
        standard.family_plan().settings(),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    let foreign_accepted = DiagnosticPlan::new(
        family,
        probes,
        profile.residual_times(),
        standard.diagnostic_settings(),
        CAP,
    )
    .unwrap();
    assert!(matches!(
        DiagnosticExportPlan::new(&profile, foreign_accepted, CAP),
        Err(DiagnosticExportError::InvalidReport)
    ));

    let replacement = TickClock::restore(-20, 8192, 94, 8192 - 94).unwrap();
    let manifest = [
        profile.manifest()[0],
        profile.manifest()[1],
        profile.manifest()[2],
        profile.manifest()[3],
        replacement,
        profile.manifest()[5],
        profile.manifest()[6],
    ];
    let residual = [
        profile.residual_times()[0],
        profile.residual_times()[1],
        replacement,
        profile.residual_times()[3],
    ];
    let family = standard.family_plan();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    let foreign_manifest = DiagnosticPlan::new(
        family,
        probes,
        &residual,
        standard.diagnostic_settings(),
        CAP,
    )
    .unwrap();
    assert!(matches!(
        DiagnosticExportPlan::new(&profile, foreign_manifest, CAP),
        Err(DiagnosticExportError::InvalidReport)
    ));
}

#[test]
fn diagnostic_export_keeps_pressure_n_separate_from_residual_force_m() {
    let profile = StartupProfile::new().unwrap();
    let family = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([16; 3]).unwrap(),
                workers: 0,
            },
            endpoint: 128,
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(profile.accepted_times(), 3).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(profile.manifest(), 7).unwrap(),
        7,
        CAP,
    )
    .unwrap();
    let settings = DiagnosticSettings {
        physical_samples: Layout::new([12; 3]).unwrap(),
        pressure_samples: Layout::new([24; 3]).unwrap(),
        reference_samples: Layout::new([12; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
        coverage_panels: [256, 512, 1024],
    };
    let plan =
        DiagnosticPlan::new(family, probes, profile.residual_times(), settings, CAP).unwrap();
    let export = DiagnosticExportPlan::new(&profile, plan, CAP).unwrap();
    assert_eq!(export.pressure_layout().dimensions(), [24; 3]);
    assert_eq!(export.residual_force_layout().dimensions(), [32; 3]);
}

#[test]
fn diagnostic_export_complete_profile_preserves_every_raw_finding() {
    let profile = StartupProfile::new().unwrap();
    let diagnostic = profile.plan(CAP).unwrap();
    let export = DiagnosticExportPlan::new(&profile, diagnostic, CAP).unwrap();
    let export_v2 = DiagnosticExportPlan::new_v2(&profile, diagnostic, CAP).unwrap();
    let mut mismatched_settings = diagnostic.diagnostic_settings();
    mismatched_settings.physical_floors[0] *= 2.0;
    let mismatched_diagnostic = DiagnosticPlan::new(
        diagnostic.family_plan(),
        diagnostic.probe_plan(),
        diagnostic.residual_times(),
        mismatched_settings,
        CAP,
    )
    .unwrap();
    let mismatched_export =
        DiagnosticExportPlan::new_v2(&profile, mismatched_diagnostic, CAP).unwrap();
    let mut mismatched_pressure_settings = diagnostic.diagnostic_settings();
    mismatched_pressure_settings.pressure_floors[0] *= 2.0;
    let mismatched_pressure_diagnostic = DiagnosticPlan::new(
        diagnostic.family_plan(),
        diagnostic.probe_plan(),
        diagnostic.residual_times(),
        mismatched_pressure_settings,
        CAP,
    )
    .unwrap();
    let mismatched_pressure_export =
        DiagnosticExportPlan::new_v2(&profile, mismatched_pressure_diagnostic, CAP).unwrap();
    let mut mismatched_coverage_settings = diagnostic.diagnostic_settings();
    mismatched_coverage_settings.coverage_panels = [128, 256, 512];
    let mismatched_coverage_diagnostic = DiagnosticPlan::new(
        diagnostic.family_plan(),
        diagnostic.probe_plan(),
        diagnostic.residual_times(),
        mismatched_coverage_settings,
        CAP,
    )
    .unwrap();
    let mismatched_coverage_export =
        DiagnosticExportPlan::new_v2(&profile, mismatched_coverage_diagnostic, CAP).unwrap();
    assert_eq!(
        (export.schema_version(), export_v2.schema_version()),
        (1, 2)
    );
    assert!(DiagnosticExportPlan::new(
        &profile,
        diagnostic,
        export.bounds().maximum_output_bytes - 1
    )
    .is_err());
    let mut driver = DiagnosticDriver::new(diagnostic).unwrap();
    while driver.advance().unwrap().is_some() {}
    let ordinary_clocks =
        std::array::from_fn::<_, 6, _>(|i| driver.ordinary().branch(i).unwrap().state().clock());
    let probe_clocks =
        std::array::from_fn::<_, 6, _>(|i| driver.probes().branch(i).unwrap().state().clock());
    let mut untouched = Vec::new();
    assert!(matches!(
        write_json(export, &driver.reports()[..6], &mut untouched),
        Err(DiagnosticExportError::InvalidReport)
    ));
    assert!(untouched.is_empty());
    assert!(matches!(
        write_json(mismatched_export, driver.reports(), &mut untouched),
        Err(DiagnosticExportError::InvalidReport)
    ));
    assert!(untouched.is_empty());
    assert!(matches!(
        write_json(mismatched_pressure_export, driver.reports(), &mut untouched),
        Err(DiagnosticExportError::InvalidReport)
    ));
    assert!(untouched.is_empty());
    assert!(matches!(
        write_json(mismatched_coverage_export, driver.reports(), &mut untouched),
        Err(DiagnosticExportError::InvalidReport)
    ));
    assert!(untouched.is_empty());
    assert_eq!(driver.reports().len(), 7);
    let mut bytes = Vec::new();
    let work = write_json(export, driver.reports(), &mut bytes).unwrap();
    assert_eq!(work.events, 7);
    assert_eq!(work.validation_bytes, work.output_bytes);
    assert!(work.output_bytes < export.bounds().maximum_output_bytes);
    assert!(work.validation_calls + work.output_calls <= export.bounds().maximum_write_calls);
    assert_eq!(
        ordinary_clocks,
        std::array::from_fn(|i| driver.ordinary().branch(i).unwrap().state().clock())
    );
    assert_eq!(
        probe_clocks,
        std::array::from_fn(|i| driver.probes().branch(i).unwrap().state().clock())
    );

    let decoded: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded["schema_version"], 1);
    assert!(decoded["events"][0].get("reconstructed_physical").is_none());
    assert!(decoded["events"][0]
        .get("reconstructed_reference")
        .is_none());
    assert!(decoded["context"]["coordinator_reservation"]
        .get("reconstructed_physical_work")
        .is_none());
    assert!(decoded["context"]["coordinator_reservation"]
        .get("reconstructed_pressure_work")
        .is_none());
    assert!(decoded["context"]["coordinator_reservation"]
        .get("reconstructed_reference_work")
        .is_none());
    assert!(decoded["context"].get("coverage_panels").is_none());
    assert!(decoded["context"]["coordinator_reservation"]
        .get("nominal_coverage_work")
        .is_none());
    assert!(decoded["events"][0]["accepted"]["sample"]
        .get("nominal_coverage")
        .is_none());
    assert_eq!(decoded["scientific_status"], "UnqualifiedDiagnostic");
    assert_eq!(
        decoded["context"]["case_sha256"],
        nsbu_benchmarks::CASE_SHA256
    );
    assert_eq!(
        decoded["context"]["family_identity"],
        decoded["events"][0]["family_identity"]
    );
    assert_eq!(
        decoded["context"]["probe_identity"],
        decoded["events"][0]["probe_identity"]
    );
    assert_eq!(decoded["context"]["accepted_clocks"][2]["elapsed"], "128");
    assert_eq!(
        decoded["context"]["missing_channels"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        decoded["events"].as_array().unwrap().len(),
        driver.reports().len()
    );

    v2_diagnostic_export_oracle::document(&decoded, driver.reports());

    let mut v2_bytes = Vec::new();
    write_json(export_v2, driver.reports(), &mut v2_bytes).unwrap();
    let decoded_v2: Value = serde_json::from_slice(&v2_bytes).unwrap();
    assert_eq!(decoded_v2["schema_version"], 2);
    assert!(decoded_v2["events"][0]
        .get("reconstructed_physical")
        .is_some());
    let v2_events = decoded_v2["events"].as_array().unwrap();
    assert_eq!(v2_events.len(), 7);
    assert!(v2_events
        .iter()
        .all(|event| event.get("reconstructed_pressure").is_some()));
    assert!(v2_events
        .iter()
        .all(|event| event.get("reconstructed_reference").is_some()));
    assert_eq!(
        decoded_v2["context"]["coverage_panels"],
        serde_json::json!([256, 512, 1024])
    );
    assert_eq!(
        v2_events
            .iter()
            .filter(|event| event["accepted"]["sample"]
                .get("nominal_coverage")
                .is_some())
            .count(),
        3
    );
    assert_eq!(
        decoded_v2["context"]["coordinator_reservation"]["reconstructed_physical_work"]["attempts"],
        diagnostic.bounds().probe_physical.attempts.to_string()
    );
    let pressure_work = diagnostic.bounds().probe_pressure;
    let pressure_reservation =
        &decoded_v2["context"]["coordinator_reservation"]["reconstructed_pressure_work"];
    assert_eq!(
        pressure_reservation["attempts"],
        pressure_work.attempts.to_string()
    );
    assert_eq!(
        pressure_reservation["provider_work_units"],
        pressure_work.provider_work_units.to_string()
    );
    assert_eq!(
        pressure_reservation["scalar_transforms"],
        pressure_work.scalar_transforms.to_string()
    );
    assert_eq!(
        pressure_reservation["weighted_visits"],
        pressure_work.weighted_visits.to_string()
    );
    assert_eq!(
        pressure_reservation["binding_checks"],
        pressure_work.binding_checks.to_string()
    );
    let reference_work = diagnostic.bounds().probe_reference;
    let reference_reservation =
        &decoded_v2["context"]["coordinator_reservation"]["reconstructed_reference_work"];
    assert_eq!(
        reference_reservation["attempts"],
        reference_work.attempts.to_string()
    );
    assert_eq!(
        reference_reservation["reference_evaluations"],
        reference_work.reference_evaluations.to_string()
    );
    assert_eq!(
        reference_reservation["root_iterations"],
        reference_work.root_iterations.to_string()
    );
    assert_eq!(
        reference_reservation["scalar_transforms"],
        reference_work.scalar_transforms.to_string()
    );
    assert_eq!(
        reference_reservation["weighted_visits"],
        reference_work.weighted_visits.to_string()
    );
    assert_eq!(
        reference_reservation["binding_checks"],
        reference_work.binding_checks.to_string()
    );
    let coverage_work = diagnostic.bounds().coverage;
    let coverage_reservation =
        &decoded_v2["context"]["coordinator_reservation"]["nominal_coverage_work"];
    assert_eq!(
        coverage_reservation["attempts"],
        coverage_work.attempts.to_string()
    );
    assert_eq!(
        coverage_reservation["geometry_evaluations"],
        coverage_work.geometry_evaluations.to_string()
    );
    assert_eq!(
        coverage_reservation["metadata_visits"],
        coverage_work.metadata_visits.to_string()
    );
    v2_diagnostic_export_oracle::document(&decoded_v2, driver.reports());

    let mut failed = FailAfter { remaining: 97 };
    match write_json(export, driver.reports(), &mut failed).unwrap_err() {
        DiagnosticExportError::Io {
            validation_bytes,
            validation_calls,
            bytes_written,
            write_calls,
            source,
        } => {
            assert_eq!(validation_bytes, work.validation_bytes);
            assert_eq!(validation_calls, work.validation_calls);
            assert_eq!(bytes_written, 97);
            assert!(write_calls > 0);
            assert_eq!(source.raw_os_error(), Some(28));
        }
        error => panic!("unexpected error: {error:?}"),
    }
}

struct FailAfter {
    remaining: usize,
}
impl Write for FailAfter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::from_raw_os_error(28));
        }
        let count = self.remaining.min(bytes.len());
        self.remaining -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
