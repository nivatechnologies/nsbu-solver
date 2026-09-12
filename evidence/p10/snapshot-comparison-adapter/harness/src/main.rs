mod absolute;
mod compare;
mod decode;
mod model;

use std::{env, ffi::OsString, path::PathBuf};

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    println!("{}", run(&args)?);
    Ok(())
}

fn run(args: &[OsString]) -> Result<String, String> {
    if args.len() != 3 {
        return Err("usage: p10-snapshot-comparison-adapter LEFT.json RIGHT.json CAP_BYTES".into());
    }
    let cap = args[2]
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(model::debug)?;
    let left_manifest = decode::read_manifest(&PathBuf::from(&args[0]))?;
    let right_manifest = decode::read_manifest(&PathBuf::from(&args[1]))?;
    let admitted = decode::preflight(&left_manifest, &right_manifest)?;
    if admitted > cap {
        return Err(format!(
            "comparison reservation {admitted} exceeds cap {cap}"
        ));
    }
    let left = decode::load(&left_manifest)?;
    let right = decode::load(&right_manifest)?;
    let output = compare::compare(&left_manifest, &left, &right_manifest, &right, admitted)?;
    serde_json::to_string_pretty(&output).map_err(model::debug)
}

#[cfg(test)]
mod tests;
