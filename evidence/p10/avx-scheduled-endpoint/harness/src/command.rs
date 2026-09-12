//! Exact two-argument harness command parser.
use nsbu_solver::SolverError;
use std::path::PathBuf;

pub enum Command {
    Preflight,
    Run(PathBuf),
}

pub fn parse() -> Result<Command, SolverError> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [mode, path] = arguments.as_slice() else {
        return Err(SolverError::InvalidPayload);
    };
    command_for(mode, path)
}

fn command_for(mode: &str, path: &str) -> Result<Command, SolverError> {
    match mode {
        "preflight" => Ok(Command::Preflight),
        "run" => Ok(Command::Run(PathBuf::from(path))),
        _ => Err(SolverError::InvalidPayload),
    }
}
