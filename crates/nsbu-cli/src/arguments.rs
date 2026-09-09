use std::ffi::OsString;

pub(crate) enum Command {
    Help,
    Version,
    Invalid,
}

pub(crate) fn parse(arguments: &[OsString]) -> Command {
    match arguments {
        [] => Command::Help,
        [argument] if argument == "--help" || argument == "-h" => Command::Help,
        [argument] if argument == "--version" || argument == "-V" => Command::Version,
        _ => Command::Invalid,
    }
}
