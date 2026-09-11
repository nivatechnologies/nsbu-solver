//! Independent JSON decoding, complete-value, refusal, and writer-failure controls.
use nsbu_benchmarks::v2_experiment::diagnostic::{
    export::{write_json, DiagnosticExportError, DiagnosticExportPlan},
    DiagnosticDriver, StartupProfile,
};
mod v2_diagnostic_export_oracle;
use serde_json::Value;
use std::io::{self, Write};

const CAP: usize = 256 * 1024 * 1024;

#[test]
fn diagnostic_export_complete_profile_preserves_every_raw_finding() {
    let profile = StartupProfile::new().unwrap();
    let diagnostic = profile.plan(CAP).unwrap();
    let export = DiagnosticExportPlan::new(&profile, diagnostic, CAP).unwrap();
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
