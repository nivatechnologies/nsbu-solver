use std::ffi::OsString;

pub(crate) enum Command {
    Help,
    Version,
    Smooth(Args),
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
        [command, rest @ ..] if command == "smooth" => smooth(rest),
        _ => Command::Invalid,
    }
}

fn smooth(items: &[OsString]) -> Command {
    let mut args = Args {
        grid: 8,
        method: MethodName::CoxMatthews,
        step_ticks: 64,
        endpoint_ticks: 256,
        tick_exponent: -16,
        maximum_attempts: 4,
        memory_cap: 64 * 1024 * 1024,
        dry_run: false,
    };
    let mut index = 0;
    while index < items.len() {
        let Some(name) = items[index].to_str() else {
            return Command::Invalid;
        };
        if name == "--dry-run" {
            if args.dry_run {
                return Command::Invalid;
            }
            args.dry_run = true;
            index += 1;
            continue;
        }
        let Some(value) = items.get(index + 1).and_then(|item| item.to_str()) else {
            return Command::Invalid;
        };
        if !set_option(&mut args, name, value) {
            return Command::Invalid;
        }
        index += 2;
    }
    Command::Smooth(args)
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
        _ => false,
    }
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
