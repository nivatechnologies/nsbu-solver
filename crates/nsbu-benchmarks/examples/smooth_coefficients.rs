//! Export an actual independently evolved smooth state as exact binary64 words.
//! Run with `4 CM` (also N=8,12 and HO). A successful JSON export is diagnostic evidence.
mod arithmetic_support;
use arithmetic_support::{ExportError, CAP};
use nsbu_benchmarks::smooth_run::ReconstructedRun;
use nsbu_solver::{integrators::method::Method, SolverError};
use std::io::{self, Write};

fn main() -> Result<(), ExportError> {
    let mut args = std::env::args().skip(1);
    let n = arithmetic_support::grid(args.next())?;
    let method = arithmetic_support::method(args.next())?;
    if args.next().is_some() {
        return Err(SolverError::InvalidPayload.into());
    }
    export(&mut io::stdout().lock(), n, method, CAP)
}

fn export(out: &mut impl Write, n: usize, method: Method, cap: usize) -> Result<(), ExportError> {
    let run = arithmetic_support::evolution::evolve(n, method, cap)?;
    write_state(out, &run)
}

fn write_state(out: &mut impl Write, run: &ReconstructedRun) -> Result<(), ExportError> {
    let domain = run.state().plan().domain();
    let n = domain.layout().dimensions()[0];
    let method = run.history().controller().configuration().method;
    let elapsed = run.state().clock().elapsed();
    let committed = run.history().controller().committed();
    let calls: usize = run.work().iter().map(|work| work.calls()).sum();
    writeln!(out, "{{\"grid\":{n},\"method\":\"{method:?}\",\"tick_exponent\":-16,\"elapsed_ticks\":{elapsed},\"macro_step_ticks\":16,\"macro_steps\":{committed},\"fine_steps_committed\":{},\"rhs_calls\":{calls},\"reference_assignments\":0,\"accepted_pde_windows\":0,\"state\":[", 2*committed)?;
    arithmetic_support::field(
        out,
        domain,
        [
            run.state().component(0)?,
            run.state().component(1)?,
            run.state().component(2)?,
        ],
    )?;
    writeln!(out, "]}}")?;
    out.flush()?;
    Ok(())
}

#[test]
fn both_methods_export_actual_full_states_and_refuse_an_unreserved_owner() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut output = Vec::new();
        export(&mut output, 4, method, CAP).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("\"elapsed_ticks\":128"));
        assert!(text.contains("\"macro_steps\":8,\"fine_steps_committed\":16"));
        assert!(text.contains(&format!("\"rhs_calls\":{}", 8 * method.rhs_calls())));
        assert_eq!(text.matches("\"mode\"").count(), 48);
        assert!(text.ends_with("]}\n"));
    }
    let mut output = Vec::new();
    assert!(export(&mut output, 4, Method::CoxMatthews, 1).is_err());
    assert!(output.is_empty());
}

#[test]
fn a_closed_output_is_an_explicit_failure() {
    let mut no_room = &mut [][..];
    assert!(matches!(
        export(&mut no_room, 4, Method::CoxMatthews, CAP),
        Err(ExportError::Io(_))
    ));
}
