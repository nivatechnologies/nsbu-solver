#[path = "../../../snapshot-comparison-adapter/harness/src/absolute.rs"]
mod absolute;
mod bridge;
#[path = "../../../snapshot-comparison-adapter/harness/src/decode.rs"]
mod decode;
#[path = "../../../snapshot-comparison-adapter/harness/src/model.rs"]
#[allow(dead_code)]
mod model;

use std::{env, ffi::OsString, path::PathBuf};

fn main() -> Result<(), String> {
    print!("{}", run(&env::args_os().skip(1).collect::<Vec<_>>())?);
    Ok(())
}

fn run(args: &[OsString]) -> Result<String, String> {
    if !(args.len() == 2 || (args.len() == 3 && args[2] == "--execute")) {
        return Err(
            "usage: p10-external-reference-bridge BRIDGE.json CAP_BYTES [--execute]".into(),
        );
    }
    let cap = args[1]
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(model::debug)?;
    let input = bridge::bind(&PathBuf::from(&args[0]))?;
    if input.preflight.storage.total > cap {
        return Err(format!(
            "bridge reservation {} exceeds cap {cap}",
            input.preflight.storage.total
        ));
    }
    if args.len() == 2 {
        bridge::write_json(&bridge::preflight_output(&input))
    } else {
        bridge::write_json(&bridge::execute(&input, cap)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_requires_explicit_execute_and_cap() {
        assert!(run(&[]).unwrap_err().contains("usage"));
        assert!(run(&["x".into(), "bad".into()])
            .unwrap_err()
            .contains("InvalidDigit"));
    }
}
