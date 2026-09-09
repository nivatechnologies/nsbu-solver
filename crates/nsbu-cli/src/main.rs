//! NSBU Solver command-line entry point.
mod arguments;

use arguments::{parse, Command};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match parse(&arguments) {
        Command::Help => println!(
            "{}\nUsage: nsbu [--help | --version]\n\nNumerical simulation commands are not implemented yet.",
            nsbu_solver::NAME
        ),
        Command::Version => println!("{} {}", nsbu_solver::NAME, nsbu_solver::VERSION),
        Command::Invalid => {
            eprintln!("Unsupported arguments. Use nsbu --help for available commands.");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
