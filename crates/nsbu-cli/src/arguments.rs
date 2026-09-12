use std::{ffi::OsString, path::PathBuf};

pub(crate) enum Command {
    Help,
    Version,
    Smooth(Args),
    Resume(Args),
    V2(Args),
    ResumeV2(Args),
    Invalid,
}

pub(crate) struct Args {
    pub grid: usize,
    pub method: MethodName,
    pub step_ticks: u128,
    pub endpoint_ticks: u128,
    pub tick_exponent: i32,
    pub maximum_attempts: usize,
    pub memory_cap: usize,
    pub dry_run: bool,
    pub cache_force: bool,
    pub checkpoint: Option<PathBuf>,
    pub checkpoint_after: Option<usize>,
    pub force_grid: Option<usize>,
    pub workers: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum MethodName {
    CoxMatthews,
    HochbruckOstermann,
}

pub(crate) fn parse(arguments: &[OsString]) -> Command {
    match arguments {
        [] => Command::Help,
        [argument] if argument == "--help" || argument == "-h" => Command::Help,
        [argument] if argument == "--version" || argument == "-V" => Command::Version,
        [command, argument]
            if is_command(command) && (argument == "--help" || argument == "-h") =>
        {
            Command::Help
        }
        [command, rest @ ..] if command == "smooth" => profile(rest, false, false),
        [command, rest @ ..] if command == "resume" => profile(rest, false, true),
        [command, rest @ ..] if command == "v2" => profile(rest, true, false),
        [command, rest @ ..] if command == "resume-v2" => profile(rest, true, true),
        _ => Command::Invalid,
    }
}

fn is_command(command: &OsString) -> bool {
    ["smooth", "resume", "v2", "resume-v2"]
        .iter()
        .any(|name| command == name)
}

fn defaults(concentrating: bool) -> Args {
    let [grid, step, endpoint, attempts] = if concentrating {
        [4, 128, 4096, 32]
    } else {
        [8, 64, 256, 4]
    };
    Args {
        grid,
        step_ticks: step as u128,
        endpoint_ticks: endpoint as u128,
        maximum_attempts: attempts,
        tick_exponent: if concentrating { -20 } else { -16 },
        method: MethodName::CoxMatthews,
        memory_cap: 64 * 1024 * 1024,
        dry_run: false,
        cache_force: false,
        checkpoint: None,
        checkpoint_after: None,
        force_grid: None,
        workers: 0,
    }
}

fn profile(items: &[OsString], concentrating: bool, resume: bool) -> Command {
    let Some(args) = parse_options(items, concentrating) else {
        return Command::Invalid;
    };
    if resume {
        if !args.cache_force
            && (args.dry_run || args.checkpoint.is_none() || args.checkpoint_after.is_some())
        {
            return Command::Invalid;
        }
        if concentrating {
            Command::ResumeV2(args)
        } else {
            Command::Resume(args)
        }
    } else {
        if !args.cache_force && args.checkpoint.is_some() != args.checkpoint_after.is_some() {
            return Command::Invalid;
        }
        if concentrating {
            Command::V2(args)
        } else {
            Command::Smooth(args)
        }
    }
}

fn parse_options(items: &[OsString], concentrating: bool) -> Option<Args> {
    let mut args = defaults(concentrating);
    let mut index = 0;
    while index < items.len() {
        let name = items[index].to_str()?;
        if name == "--dry-run" {
            if args.dry_run {
                return None;
            }
            args.dry_run = true;
            index += 1;
        } else if name == "--cache-force" {
            if !concentrating || args.cache_force {
                return None;
            }
            args.cache_force = true;
            index += 1;
        } else {
            let value = items.get(index + 1)?.to_str()?;
            if !set_profile_option(&mut args, name, value, concentrating) {
                return None;
            }
            index += 2;
        }
    }
    Some(args)
}

fn set_profile_option(args: &mut Args, name: &str, value: &str, concentrating: bool) -> bool {
    match name {
        "--force-grid" => concentrating && set_optional_grid(value, args),
        "--workers" => concentrating && set(value, &mut args.workers),
        _ => set_option(args, name, value),
    }
}

fn set_option(args: &mut Args, name: &str, value: &str) -> bool {
    match name {
        "--grid" | "--domain" => set(value, &mut args.grid),
        "--step-ticks" | "--step" => set(value, &mut args.step_ticks),
        "--target" | "--endpoint-ticks" => set(value, &mut args.endpoint_ticks),
        "--tick-exponent" | "--tick" => set(value, &mut args.tick_exponent),
        "--attempts" | "--attempt-cap" => set(value, &mut args.maximum_attempts),
        "--memory-cap" => set(value, &mut args.memory_cap),
        "--method" => set_method(value, args),
        "--checkpoint" | "--checkpoint-file" => set_path(value, args),
        "--checkpoint-after" | "--checkpoint-steps" => set_checkpoint_after(value, args),
        _ => false,
    }
}

fn set_optional_grid(value: &str, args: &mut Args) -> bool {
    if args.force_grid.is_some() {
        return false;
    }
    match value.parse() {
        Ok(value) => {
            args.force_grid = Some(value);
            true
        }
        Err(_) => false,
    }
}

fn set_checkpoint_after(value: &str, args: &mut Args) -> bool {
    if args.checkpoint_after.is_some() {
        return false;
    }
    match value.parse() {
        Ok(value) => {
            args.checkpoint_after = Some(value);
            true
        }
        Err(_) => false,
    }
}

fn set_path(value: &str, args: &mut Args) -> bool {
    if value.is_empty() || value.len() > 4096 || args.checkpoint.is_some() {
        return false;
    }
    args.checkpoint = Some(PathBuf::from(value));
    true
}

fn set<T: std::str::FromStr>(value: &str, target: &mut T) -> bool {
    match value.parse() {
        Ok(parsed) => {
            *target = parsed;
            true
        }
        Err(_) => false,
    }
}

fn set_method(value: &str, args: &mut Args) -> bool {
    args.method = match value {
        "cm" => MethodName::CoxMatthews,
        "ho" => MethodName::HochbruckOstermann,
        _ => return false,
    };
    true
}
