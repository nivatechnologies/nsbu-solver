//! Export complete actual velocity derivatives, vorticity and independently constructed pressure.
mod arithmetic_support;
mod derived_support;
use arithmetic_support::{ExportError, CAP};
use nsbu_solver::{integrators::method::Method, SolverError};
use std::io::{self, Write};
fn main() -> Result<(), ExportError> {
    execute(std::env::args().skip(1), &mut io::stdout().lock())
}
fn execute(
    mut args: impl Iterator<Item = String>,
    out: &mut impl Write,
) -> Result<(), ExportError> {
    let n = arithmetic_support::grid(args.next())?;
    let method = arithmetic_support::method(args.next())?;
    let dry_run = match args.next().as_deref() {
        None => false,
        Some("--dry-run") => true,
        _ => return Err(SolverError::InvalidPayload.into()),
    };
    if args.next().is_some() {
        return Err(SolverError::InvalidPayload.into());
    }
    if dry_run {
        write_preflight(out, n, method, CAP)
    } else {
        export(out, n, method, CAP)
    }
}

fn write_preflight(
    out: &mut impl Write,
    n: usize,
    method: Method,
    cap: usize,
) -> Result<(), ExportError> {
    let plan = arithmetic_support::evolution::plan(n, method, cap)?;
    let diagnostic_bytes = derived_support::Workspace::reservation(plan.resources().domain())?;
    let joint = plan
        .resources()
        .total()
        .checked_add(diagnostic_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    if joint > cap {
        return Err(SolverError::ResourceLimit.into());
    }
    writeln!(out,"{{\"case\":\"CyclicSine\",\"grid\":{n},\"samples\":{},\"method\":\"{method:?}\",\"tick_exponent\":-16,\"endpoint_ticks\":128,\"macro_step_ticks\":16,\"maximum_attempts\":8,\"rhs_calls\":{},\"diagnostic_scalar_transforms\":55,\"owner_bytes\":{},\"diagnostic_bytes\":{diagnostic_bytes},\"reserved_bytes\":{joint},\"cap_bytes\":{cap},\"accepted_pde_windows\":0}}",2*n,8*method.rhs_calls(),plan.resources().total())?;
    Ok(())
}
fn export(out: &mut impl Write, n: usize, method: Method, cap: usize) -> Result<(), ExportError> {
    let plan = arithmetic_support::evolution::plan(n, method, cap)?;
    let source = plan.resources().domain();
    let diagnostic_bytes = derived_support::Workspace::reservation(source)?;
    let joint = plan
        .resources()
        .total()
        .checked_add(diagnostic_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    if joint > cap {
        return Err(SolverError::ResourceLimit.into());
    }
    let run = arithmetic_support::evolution::evolve(n, method, cap)?;
    let mut workspace = derived_support::Workspace::new(source, diagnostic_bytes)?;
    workspace.evaluate(&run)?;
    workspace.write(out, &run, joint, cap)
}

#[test]
fn both_methods_stream_complete_inventory_from_their_actual_rest_trajectories() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut out = Vec::new();
        let name = if method == Method::CoxMatthews {
            "CM"
        } else {
            "HO"
        };
        execute(["4", name].into_iter().map(str::to_owned), &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s.matches("\"quantity\"").count(), 46);
        assert_eq!(s.matches("\"quantity\":\"velocity\"").count(), 39);
        assert!(s.contains("\"reference_assignments\":0"));
        assert!(s.ends_with("]}\n"));
    }
}
#[test]
fn caps_and_closed_outputs_are_explicit_failures() {
    let mut out = Vec::new();
    assert!(export(&mut out, 12, Method::CoxMatthews, 1).is_err());
    assert!(out.is_empty());
    let mut closed = &mut [][..];
    assert!(matches!(
        export(&mut closed, 4, Method::CoxMatthews, CAP),
        Err(ExportError::Io(_))
    ));
}

#[test]
fn aggregate_preflight_and_diagnostic_domain_binding_are_checked() {
    let mut out = Vec::new();
    execute(
        ["12", "CM", "--dry-run"].into_iter().map(str::to_owned),
        &mut out,
    )
    .unwrap();
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("\"diagnostic_scalar_transforms\":55"));
    let mut out = Vec::new();
    assert!(write_preflight(&mut out, 12, Method::CoxMatthews, 1).is_err());
    assert!(out.is_empty());
    for arguments in [
        vec!["4", "CM", "bad"],
        vec!["4", "CM", "--dry-run", "extra"],
        vec!["4"],
    ] {
        let mut out = Vec::new();
        assert!(execute(arguments.into_iter().map(str::to_owned), &mut out).is_err());
        assert!(out.is_empty());
    }
    let source = nsbu_solver::domain::Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let bytes = derived_support::Workspace::reservation(source).unwrap();
    assert!(derived_support::Workspace::new(source, bytes - 1).is_err());
    let different = nsbu_solver::domain::Domain::new([4; 3], [1.0; 3], 2.0).unwrap();
    let mut workspace = derived_support::Workspace::new(different, CAP).unwrap();
    let run = arithmetic_support::evolution::evolve(4, Method::CoxMatthews, CAP).unwrap();
    assert!(workspace.evaluate(&run).is_err());
    assert!(arithmetic_support::evolution::plan(16, Method::CoxMatthews, CAP).is_err());
}
