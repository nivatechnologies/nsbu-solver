//! Export all 33 exact-stage raw force arrays as binary64 words for controlled arithmetic runs.
mod arithmetic_support;
use arithmetic_support::{ExportError, CAP};
use nsbu_benchmarks::smooth::CyclicSine;
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};
use std::io::{self, Write};

fn main() -> Result<(), ExportError> {
    let mut args = std::env::args().skip(1);
    let n = arithmetic_support::grid(args.next())?;
    if args.next().is_some() {
        return Err(SolverError::InvalidPayload.into());
    }
    export(&mut io::stdout().lock(), n, CAP)
}

fn export(out: &mut impl Write, n: usize, cap: usize) -> Result<(), ExportError> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0)?;
    let mut force = CyclicSine::new(domain)?;
    let limit = force.limits().ok_or(SolverError::UnknownProviderCost)?;
    let len = domain.layout().half_len();
    let bytes = len
        .checked_mul(3 * std::mem::size_of::<Complex64>())
        .and_then(|n| n.checked_add(limit.storage_bytes))
        .ok_or(SolverError::SizeOverflow)?;
    if bytes > cap {
        return Err(SolverError::ResourceLimit.into());
    }
    let mut values: [Vec<Complex64>; 3] = std::array::from_fn(|_| Vec::new());
    for field in &mut values {
        field
            .try_reserve_exact(len)
            .map_err(|_| SolverError::AllocationFailed)?;
        field.resize(len, Complex64::new(0.0, 0.0));
    }
    // The fixed 33-call schedule bounds all provider work; serialization is streamed.
    writeln!(out, "{{\"grid\":{n},\"tick_exponent\":-16,\"samples\":[")?;
    for tick in (0..=128).step_by(4) {
        force.evaluate(
            TickClock::restore(-16, 512, tick, 512 - tick)?,
            limit,
            values.each_mut().map(Vec::as_mut_slice),
        )?;
        writeln!(out, "{{\"tick\":{tick},\"force\":[")?;
        arithmetic_support::field(out, domain, values.each_ref().map(Vec::as_slice))?;
        writeln!(out, "]}}{}", if tick == 128 { "" } else { "," })?;
    }
    writeln!(out, "]}}")?;
    out.flush()?;
    Ok(())
}

#[test]
fn all_exact_stage_clocks_and_all_coefficients_are_exported() {
    let mut output = Vec::new();
    export(&mut output, 4, CAP).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(text.matches("\"tick\":").count(), 33);
    assert_eq!(text.matches("\"mode\":").count(), 33 * 48);
    assert!(text.contains("\"tick\":128"));
    assert!(text.contains("4621537642612260864")); // exact 19/2 startup amplitude
    assert!(text.ends_with("]}\n"));
}

#[test]
fn storage_and_output_refusals_are_explicit() {
    let mut output = Vec::new();
    assert!(export(&mut output, 4, 1).is_err());
    assert!(output.is_empty());
    let mut no_room = &mut [][..];
    assert!(matches!(
        export(&mut no_room, 4, CAP),
        Err(ExportError::Io(_))
    ));
}
