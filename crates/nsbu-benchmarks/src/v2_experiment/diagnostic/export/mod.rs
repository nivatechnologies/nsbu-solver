//! Versioned, bounded JSON export of complete unqualified diagnostic events.
mod context;
mod event;
mod findings;
mod json;
mod plan;
mod values;

use self::json::Json;
use std::io::{self, Write};

pub use plan::{DiagnosticExportBounds, DiagnosticExportPlan, DiagnosticExportWork};

/// Export refusal. Writer failures may leave exactly `bytes_written` prefix bytes.
#[derive(Debug)]
pub enum DiagnosticExportError {
    /// Profile, plan, report order, identity, schedule, or finite-value validation failed.
    InvalidReport,
    /// Checked reservation arithmetic overflowed.
    SizeOverflow,
    /// The caller's output cap is below the complete conservative reservation.
    ResourceLimit,
    /// The caller-owned writer failed after a possibly nonempty valid JSON prefix.
    Io {
        /// Bytes consumed by the completed validation pass.
        validation_bytes: usize,
        /// Calls consumed by the completed validation pass.
        validation_calls: usize,
        /// Bytes successfully accepted by the writer before its failure.
        bytes_written: usize,
        /// Calls made to the caller writer, including the failed call.
        write_calls: usize,
        /// Original writer error.
        source: io::Error,
    },
}

/// Emit one complete JSON document. Reports and numerical state are only borrowed.
///
/// All content is validated with a counting sink before the caller writer is touched.
/// A subsequent I/O failure is not transactional: the writer retains the reported prefix.
pub fn write_json(
    plan: DiagnosticExportPlan,
    reports: &[super::DiagnosticEvent],
    writer: &mut impl Write,
) -> Result<DiagnosticExportWork, DiagnosticExportError> {
    if reports.len() != plan.event_count() {
        return Err(DiagnosticExportError::InvalidReport);
    }
    let mut validation = Json::new(io::sink());
    event::document(&mut validation, plan, reports)?;
    let (validation_bytes, validation_calls, _) = validation.finish()?;
    if validation_bytes > plan.bounds().maximum_output_bytes {
        return Err(DiagnosticExportError::SizeOverflow);
    }
    let mut output = Json::new(writer);
    if let Err(error) = event::document(&mut output, plan, reports) {
        return Err(match error {
            DiagnosticExportError::Io {
                bytes_written,
                write_calls,
                source,
                ..
            } => DiagnosticExportError::Io {
                validation_bytes,
                validation_calls,
                bytes_written,
                write_calls,
                source,
            },
            other => other,
        });
    }
    let (output_bytes, output_calls, _) = output.finish()?;
    Ok(DiagnosticExportWork {
        validation_bytes,
        output_bytes,
        validation_calls,
        output_calls,
        events: reports.len(),
    })
}
