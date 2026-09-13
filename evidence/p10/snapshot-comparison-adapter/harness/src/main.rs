mod absolute;
mod compare;
mod decode;
mod hessian;
mod mixed;
mod model;

use std::{env, ffi::OsString, path::PathBuf};

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    println!("{}", run(&args)?);
    Ok(())
}

fn run(args: &[OsString]) -> Result<String, String> {
    if args.len() == 5 && args[4] == "--mixed-force-space" {
        return run_mixed(args);
    }
    run_pair(args)
}

fn run_pair(args: &[OsString]) -> Result<String, String> {
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: p10-snapshot-comparison-adapter LEFT.json RIGHT.json CAP_BYTES [--ordered-hessian]"
                .into(),
        );
    }
    let ordered_hessian = ordered_hessian_requested(args)?;
    let cap = args[2]
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(model::debug)?;
    let left_manifest = decode::read_manifest(&PathBuf::from(&args[0]))?;
    let right_manifest = decode::read_manifest(&PathBuf::from(&args[1]))?;
    validate_requested_pair(ordered_hessian, &left_manifest, &right_manifest)?;
    let admitted = decode::preflight(&left_manifest, &right_manifest)?;
    if admitted > cap {
        return Err(format!(
            "comparison reservation {admitted} exceeds cap {cap}"
        ));
    }
    let left = decode::load(&left_manifest)?;
    let right = decode::load(&right_manifest)?;
    if !ordered_hessian {
        return format_output(&left_manifest, &left, &right_manifest, &right, admitted);
    }
    format_hessian_output(&left_manifest, &left, &right_manifest, &right, admitted)
}

fn run_mixed(args: &[OsString]) -> Result<String, String> {
    let cap = args[3]
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(model::debug)?;
    let coarse_manifest = decode::read_manifest(&PathBuf::from(&args[0]))?;
    let baseline_manifest = decode::read_manifest(&PathBuf::from(&args[1]))?;
    let force_manifest = decode::read_manifest(&PathBuf::from(&args[2]))?;
    mixed::validate_manifests(&coarse_manifest, &baseline_manifest, &force_manifest)?;
    let admitted = decode::preflight_three(&coarse_manifest, &baseline_manifest, &force_manifest)?;
    if admitted > cap {
        return Err(format!(
            "comparison reservation {admitted} exceeds cap {cap}"
        ));
    }
    let coarse = decode::load(&coarse_manifest)?;
    let baseline = decode::load(&baseline_manifest)?;
    let force = decode::load(&force_manifest)?;
    let output = mixed::diagnostic_output(
        &coarse_manifest,
        &coarse,
        &baseline_manifest,
        &baseline,
        &force_manifest,
        &force,
        admitted,
    )?;
    serde_json::to_string_pretty(&output).map_err(model::debug)
}

fn ordered_hessian_requested(args: &[OsString]) -> Result<bool, String> {
    match args.get(3) {
        None => Ok(false),
        Some(value) if value == "--ordered-hessian" => Ok(true),
        Some(_) => Err("unsupported optional diagnostic".into()),
    }
}

fn validate_requested_pair(
    ordered_hessian: bool,
    left: &model::Manifest,
    right: &model::Manifest,
) -> Result<(), String> {
    if ordered_hessian {
        return hessian::validate_manifest_pair(left, right);
    }
    if left.comparison_kind != model::ComparisonKind::MatchedSpatial
        || right.comparison_kind != model::ComparisonKind::MatchedSpatial
    {
        return compare::validate_manifest_pair(left, right);
    }
    Ok(())
}

fn format_hessian_output(
    left_manifest: &model::Manifest,
    left: &model::Snapshot,
    right_manifest: &model::Manifest,
    right: &model::Snapshot,
    admitted: usize,
) -> Result<String, String> {
    let output = hessian::diagnostic_output(left_manifest, left, right_manifest, right, admitted)?;
    serde_json::to_string_pretty(&output).map_err(model::debug)
}

fn format_output(
    left_manifest: &model::Manifest,
    left: &model::Snapshot,
    right_manifest: &model::Manifest,
    right: &model::Snapshot,
    admitted: usize,
) -> Result<String, String> {
    match (
        left_manifest.comparison_kind,
        right_manifest.comparison_kind,
    ) {
        (model::ComparisonKind::MatchedSpatial, model::ComparisonKind::MatchedSpatial) => {
            let output = compare::compare(left_manifest, left, right_manifest, right, admitted)?;
            serde_json::to_string_pretty(&output).map_err(model::debug)
        }
        (model::ComparisonKind::TimeDiagnostic, model::ComparisonKind::TimeDiagnostic) => {
            let output =
                compare::time_diagnostic(left_manifest, left, right_manifest, right, admitted)?;
            serde_json::to_string_pretty(&output).map_err(model::debug)
        }
        (
            model::ComparisonKind::ForceResolutionDiagnostic,
            model::ComparisonKind::ForceResolutionDiagnostic,
        ) => {
            let output = compare::force_resolution_diagnostic(
                left_manifest,
                left,
                right_manifest,
                right,
                admitted,
            )?;
            serde_json::to_string_pretty(&output).map_err(model::debug)
        }
        (model::ComparisonKind::MethodDiagnostic, model::ComparisonKind::MethodDiagnostic) => {
            let output =
                compare::method_diagnostic(left_manifest, left, right_manifest, right, admitted)?;
            serde_json::to_string_pretty(&output).map_err(model::debug)
        }
        _ => Err("comparison kind mismatch".into()),
    }
}

#[cfg(test)]
mod tests;
