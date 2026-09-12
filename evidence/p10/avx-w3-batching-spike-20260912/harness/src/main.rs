mod benchmark;
mod controls;
mod model;
mod pool;
mod records;
mod util;

use stats_alloc::{StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, process::ExitCode};

#[global_allocator]
pub(crate) static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() -> ExitCode {
    match benchmark::execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal=error detail={error}");
            ExitCode::from(1)
        }
    }
}
