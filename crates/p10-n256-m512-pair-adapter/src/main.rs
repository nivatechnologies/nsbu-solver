//! p10-n256-m512-pair-adapter: separately bound N256/M512 vs N384/M512
//! clock-4096 endpoint spatial pair adapter.
//!
//! This adapter is a prerequisite artifact for the conditional fixed-M512
//! spatial discriminant. It never reuses, weakens or rebinds the reviewed
//! N384->N512 `MATCHED_M512_SPATIAL_DIAGNOSTIC` contract, and it emits an
//! endpoint diagnostic only. It performs no solver execution, staging,
//! launch, SSH, commit or qualification.
mod absolute;
mod contract;
mod decode;
mod fine_identity;
mod lineage;
mod model;
mod pair;

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    println!("{}", run(&args)?);
    Ok(())
}

fn run(args: &[OsString]) -> Result<String, String> {
    match args.first().map(|value| value.to_str()) {
        Some(Some("admit")) if args.len() == 2 => run_admit(&args[1]),
        Some(Some("compare")) if args.len() == 5 => run_compare(&args[1..]),
        _ => Err(
            "usage: p10-n256-m512-pair-adapter admit LINEAGE_INTAKE.json | \
             p10-n256-m512-pair-adapter compare COARSE.json FINE.json CAP_BYTES DEADLINE_EPOCH"
                .into(),
        ),
    }
}

fn run_admit(path: &OsString) -> Result<String, String> {
    let intake = lineage::admit_standalone(&PathBuf::from(path))?;
    Ok(format!(
        "admitted: bound clock-4096 N256/M512 lineage intake {} ({} committed states, endpoint file SHA-256 {})",
        path.to_string_lossy(),
        intake.value.states.len(),
        intake.endpoint_file_sha256
    ))
}

fn run_compare(args: &[OsString]) -> Result<String, String> {
    let cap = args[2]
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(model::debug)?;
    let deadline = args[3]
        .to_str()
        .ok_or("DEADLINE_EPOCH is not UTF-8")?
        .parse::<u64>()
        .map_err(model::debug)?;
    let left = read_coarse_manifest(&PathBuf::from(&args[0]))?;
    let right = decode::read_manifest(&PathBuf::from(&args[1]))?;
    let admitted = pair::admit(left, right, cap, deadline, pair::now_epoch()?)?;
    pair::run(admitted)
}

/// The coarse input manifest is part of a future completed capture intake. A
/// missing coarse manifest is lineage absence, never a generic I/O detail.
fn read_coarse_manifest(path: &Path) -> Result<model::Manifest, String> {
    decode::read_manifest(path).map_err(|error| {
        if !path.exists() {
            format!(
                "{}: no N256/M512 coarse input manifest exists at {}; the reviewed capture preparation records the coarse trajectory as prepared_not_executed",
                lineage::LINEAGE_ABSENT,
                path.display()
            )
        } else {
            error
        }
    })
}

#[cfg(test)]
mod tests;
