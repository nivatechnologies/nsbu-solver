#[path = "../../../snapshot-comparison-adapter/harness/src/decode.rs"]
#[allow(dead_code)]
mod decode;
#[path = "../../../snapshot-comparison-adapter/harness/src/model.rs"]
#[allow(dead_code)]
mod model;

mod cache;
mod diagnostic;
mod pilot;
mod projection;
mod projection_n512;

use std::{env, ffi::OsString, path::PathBuf};

fn main() -> Result<(), String> {
    run(&env::args_os().skip(1).collect::<Vec<_>>())
}

fn run(args: &[OsString]) -> Result<(), String> {
    match args {
        [command] if command == "pilot" => {
            print!("{}", diagnostic::json(&pilot::run()?)?);
            Ok(())
        }
        [command, manifest, cap] if command == "preflight" => {
            let cap = parse_cap(cap)?;
            let input = diagnostic::bind(&PathBuf::from(manifest), cap, false)?;
            print!("{}", diagnostic::json(&diagnostic::preflight_output(&input))?);
            Ok(())
        }
        [command] if command == "projection-preflight" => {
            print!("{}", diagnostic::json(&projection::preflight()?)?);
            Ok(())
        }
        [command] if command == "projection-n512-preflight" => {
            print!("{}", diagnostic::json(&projection_n512::preflight()?)?);
            Ok(())
        }
        [command, cap, output, review]
            if command == "projection-n512-execute" && review == "--root-reviewed" =>
        {
            projection_n512::execute(parse_cap(cap)?, &PathBuf::from(output))
        }
        [command, cap, output, review]
            if command == "projection-execute" && review == "--root-reviewed" =>
        {
            projection::execute(parse_cap(cap)?, &PathBuf::from(output))
        }
        [command, manifest, cap, output, review]
            if command == "execute" && review == "--root-reviewed" =>
        {
            let cap = parse_cap(cap)?;
            let input = diagnostic::bind(&PathBuf::from(manifest), cap, true)?;
            let report = diagnostic::execute(&input)?;
            diagnostic::write_transactional(&PathBuf::from(output), &report)
        }
        _ => Err("usage: p10-n384-regional-snapshot-diagnostic pilot | preflight SNAPSHOT.json CAP_BYTES | execute SNAPSHOT.json CAP_BYTES OUTPUT.json --root-reviewed | projection-preflight | projection-execute CAP_BYTES OUTPUT.json --root-reviewed | projection-n512-preflight | projection-n512-execute CAP_BYTES OUTPUT.json --root-reviewed".into()),
    }
}

fn parse_cap(value: &OsString) -> Result<usize, String> {
    value
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse()
        .map_err(model::debug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_refuses_implicit_execution() {
        assert!(run(&[]).unwrap_err().contains("usage"));
        assert!(run(&["execute".into()]).unwrap_err().contains("usage"));
        assert!(run(&[
            "execute".into(),
            "snapshot.json".into(),
            diagnostic::CAP_BYTES.to_string().into(),
            "output.json".into(),
            "--not-reviewed".into(),
        ])
        .unwrap_err()
        .contains("usage"));
        assert!(run(&[
            "projection-n512-execute".into(),
            "305085516888".into(),
            "output.json".into(),
            "--not-reviewed".into(),
        ])
        .unwrap_err()
        .contains("usage"));
    }
}
