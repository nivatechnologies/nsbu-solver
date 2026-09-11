//! Audit production physical reductions from complete actual-export words, without evolving a PDE.
mod reduction_support;
use nsbu_solver::SolverError;
use reduction_support::{
    input::{Packet, Plan},
    AuditError, CAP,
};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};
fn main() -> Result<(), AuditError> {
    execute(std::env::args().skip(1), &mut io::stdout().lock())
}
fn execute(mut args: impl Iterator<Item = String>, out: &mut impl Write) -> Result<(), AuditError> {
    let first = args.next().ok_or(SolverError::InvalidPayload)?;
    let second = args.next().ok_or(SolverError::InvalidPayload)?;
    let cap = args
        .next()
        .map(|s| s.parse::<usize>().map_err(|_| SolverError::InvalidPayload))
        .transpose()?
        .unwrap_or(CAP);
    if args.next().is_some() {
        return Err(SolverError::InvalidPayload.into());
    }
    if first == "--dry-run" {
        let n = second.parse().map_err(|_| SolverError::InvalidPayload)?;
        preflight(out, Plan::new(n, cap)?, cap)
    } else {
        audit(PathBuf::from(first), PathBuf::from(second), cap, out)
    }
}
fn preflight(out: &mut impl Write, plan: Plan, cap: usize) -> Result<(), AuditError> {
    writeln!(out,"{{\"status\":\"reduction-audit-preflight\",\"grid\":{},\"physical_points\":{},\"packet_bytes\":{},\"reserved_bytes\":{},\"cap_bytes\":{cap},\"accepted_pde_windows\":0}}",plan.n,plan.points,plan.packet_bytes,plan.reserved_bytes)?;
    Ok(())
}
fn audit(
    input: PathBuf,
    output: PathBuf,
    cap: usize,
    out: &mut impl Write,
) -> Result<(), AuditError> {
    let packet = Packet::read(&mut File::open(input)?, cap)?;
    let groups = reduction_support::reduce::all(&packet)?;
    // Create only after every numerical group succeeds; an existing path is always refused.
    // A later I/O error can leave a partial diagnostic file, which strict readers reject.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    reduction_support::output::write(&mut file, &packet, &groups)?;
    file.flush()?;
    preflight(out, packet.plan, cap)?;
    writeln!(out,"{{\"status\":\"reduction-audit-written\",\"quantities\":6,\"production_paths\":2,\"accepted_pde_windows\":0}}")?;
    Ok(())
}
#[test]
fn command_rejects_missing_extra_invalid_and_over_budget_arguments() {
    for args in [
        vec![],
        vec!["--dry-run"],
        vec!["--dry-run", "4", "x"],
        vec!["--dry-run", "x"],
        vec!["--dry-run", "4", "1"],
        vec!["--dry-run", "4", "67108864", "extra"],
        vec!["absent", "out"],
    ] {
        assert!(execute(args.into_iter().map(str::to_owned), &mut Vec::new()).is_err());
    }
    let mut out = Vec::new();
    execute(["--dry-run", "12"].into_iter().map(str::to_owned), &mut out).unwrap();
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("\"accepted_pde_windows\":0"));
    assert!(preflight(&mut &mut [][..], Plan::new(4, CAP).unwrap(), CAP).is_err());
    let e = AuditError::from(SolverError::InvalidPayload);
    assert!(!e.to_string().is_empty());
    let e = AuditError::from(io::Error::other("closed"));
    assert_eq!(e.to_string(), "closed");
}
#[test]
fn execution_writes_complete_new_file_and_refuses_existing_output_without_modification() {
    let root = std::env::temp_dir().join(format!("nsbu-reduction-audit-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let input = root.join("input.bin");
    let output = root.join("result.bin");
    std::fs::write(&input, reduction_support::input::fixture()).unwrap();
    let run = || {
        [
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
            CAP.to_string(),
        ]
    };
    execute(run().into_iter(), &mut Vec::new()).unwrap();
    let original = std::fs::read(&output).unwrap();
    assert_eq!(original.len(), 49640);
    assert!(execute(run().into_iter(), &mut Vec::new()).is_err());
    assert_eq!(original, std::fs::read(&output).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
