#[path = "../../../avx-w3-n256-integration-20260912/harness/src/cache.rs"]
mod cache;
mod decode;
mod model;

use cache::CachedReducedForce;
use model::{
    debug, IdentityClosureOutput, LocalizationRunOutput, NodeBinding, NodeOutput, ProbePlan,
    ReservationOutput, Snapshot, CASE_SHA256, PLAN_SHA256, PROBE, PROFILE, SOURCE, SUPPORTS,
    W3_SOURCE,
};
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace, hermite::HermiteWeights, residual::ResidualPlan,
    },
    domain::{Domain, Layout, TickClock},
    integrators::{forcing::PrescribedForce, kernel::RightHandSide, rhs::SpectralRhs},
    spectral::{modal, FftBackend, FftCatalog, W3FftIdentity, W3FftMode, W3FftPool},
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use std::{env, ffi::OsString, path::PathBuf};

type Field = [Vec<Complex64>; 3];
const FIXED_OVERHEAD_BYTES: usize = 16 * 1024 * 1024;

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    println!("{}", run(&args)?);
    Ok(())
}

fn run(args: &[OsString]) -> Result<String, String> {
    if args.len() != 3 && args.len() != 4 {
        return Err(
            "usage: p10-offline-residual-probe preflight PLAN.json CAP_BYTES | run PLAN.json SNAPSHOT_ROOT CAP_BYTES"
                .into(),
        );
    }
    let mode = args[0].to_str().ok_or("mode is not UTF-8")?;
    let plan = decode::read_plan(&PathBuf::from(&args[1]))?;
    match mode {
        "preflight" if args.len() == 3 => {
            let cap = parse_cap(&args[2])?;
            let reservation = reservations(&plan, cap)?;
            serde_json::to_string_pretty(&reservation).map_err(debug)
        }
        "run" if args.len() == 4 => {
            let cap = parse_cap(&args[3])?;
            let reservation = reservations(&plan, cap)?;
            if reservation.admitted_peak_bytes > cap {
                return Err(format!(
                    "offline probe reservation {} exceeds cap {cap}",
                    reservation.admitted_peak_bytes
                ));
            }
            let output = evaluate(&plan, PathBuf::from(&args[2]), reservation)?;
            serde_json::to_string_pretty(&output).map_err(debug)
        }
        _ => Err("invalid command shape".into()),
    }
}

fn parse_cap(value: &OsString) -> Result<usize, String> {
    value
        .to_str()
        .ok_or("CAP_BYTES is not UTF-8")?
        .parse::<usize>()
        .map_err(debug)
}

pub(crate) fn validate_plan(plan: &ProbePlan) -> Result<(), String> {
    let expected_nodes = [896, 1024, 1088, 1152, 1216, 1280, 1408];
    if plan.schema != "p10-offline-residual-probe-plan-v1"
        || plan.status != "frozen_one_probe_not_executed"
        || plan.source_commit != SOURCE
        || plan.w3_source_commit != W3_SOURCE
        || plan.case_sha256 != CASE_SHA256
        || plan.frozen_plan_sha256 != PLAN_SHA256
        || plan.profile != PROFILE
        || plan.dimensions != [384; 3]
        || plan.lengths != [1.0; 3]
        || plan.viscosity.to_bits() != 1.0_f64.to_bits()
        || plan.quantum_exponent != -20
        || plan.clock_target != 8192
        || plan.method != "cox-matthews"
        || plan.integration_force_dimensions != [384; 3]
        || plan.integration_force_workers != 32
        || plan.residual_force_dimensions != [768; 3]
        || plan.residual_force_workers != 32
        || plan.advective_limit.to_bits() != 3.3_f64.to_bits()
        || plan.probe_clock != PROBE
        || plan.supports != SUPPORTS
        || plan.nodes.len() != expected_nodes.len()
        || plan.claims.runtime_owner_imported
        || plan.claims.accepted_interpolation
        || plan.claims.acceptance_windows != 0
        || plan.claims.arithmetic != "binary64"
        || plan.claims.purpose != "read-only-offline-diagnostic"
    {
        return Err("invalid frozen offline-probe lineage".into());
    }
    let required_identity = [
        format!("source={SOURCE}"),
        format!("case={CASE_SHA256}"),
        format!("profile={PROFILE}"),
        "retained=384".into(),
        "force_samples=384".into(),
        "observer_force_samples=768".into(),
        "method=cox-matthews".into(),
    ];
    if required_identity
        .iter()
        .any(|item| !plan.snapshot_identity.contains(item))
    {
        return Err("snapshot identity omits required lineage".into());
    }
    for (&clock, node) in expected_nodes.iter().zip(&plan.nodes) {
        if node.clock != clock
            || node.epoch != clock / 64
            || node.accepted_steps != clock / 64
            || !is_hex(&node.coefficient_sha256, 64)
            || !is_hex(&node.file_sha256, 64)
        {
            return Err("invalid frozen node binding".into());
        }
    }
    for support in SUPPORTS {
        HermiteWeights::at(support.map(clock), clock(PROBE)).map_err(debug)?;
    }
    Ok(())
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn reservations(plan: &ProbePlan, cap: usize) -> Result<ReservationOutput, String> {
    let source = plan.domain()?;
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).map_err(debug)?;
    let source_state_bytes = field_bytes(source)?;
    let diagnostic_field_bytes = field_bytes(diagnostic)? / 3;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available().map_err(debug)?;
    let fft_catalog_bytes = FftCatalog::reservation(backend).map_err(debug)?;
    let integration_samples = Layout::new(plan.integration_force_dimensions).map_err(debug)?;
    let integration_force = CachedReducedForce::preflight(
        source,
        integration_samples,
        plan.integration_force_workers,
        backend,
        true,
    )
    .map_err(debug)?;
    let integration_rhs_bytes = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        source,
        integration_force,
        backend,
    )
    .map_err(debug)?;
    let rhs_w3_additional = W3FftPool::additional_reservation_with_backend(
        source.padded_layout().map_err(debug)?,
        backend,
        W3FftMode::Bidirectional,
    )
    .map_err(debug)?;
    let force_w3_additional = W3FftPool::additional_reservation_with_backend(
        integration_samples,
        backend,
        W3FftMode::Forward,
    )
    .map_err(debug)?;
    if rhs_w3_additional != 9_200_779_136 || force_w3_additional != 1_827_942_144 {
        return Err("production W3 identity does not match archived constructor".into());
    }
    let residual_samples = Layout::new(plan.residual_force_dimensions).map_err(debug)?;
    let residual_force = ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        residual_samples,
        plan.residual_force_workers,
        backend,
    )
    .map_err(debug)?;
    let residual_force_bytes = residual_force.storage_bytes;
    let conservative_workspace_bytes =
        ConservativeWorkspace::reservation_with_fft_backend(source, backend).map_err(debug)?;
    // Three fine-support values and derivatives plus reconstructed value and derivative.
    let reconstruction_peak_bytes = checked_sum(&[
        checked_mul(source_state_bytes, 8)?,
        integration_rhs_bytes,
        fft_catalog_bytes,
        FIXED_OVERHEAD_BYTES,
    ])?;
    // Reconstructed value/derivative and retained M384 force are three N384 fields. Ten
    // diagnostic components are force(3), conservative(3), base residual(3), and pressure(1).
    // Both force providers are conservatively admitted although their mutable owners are used
    // sequentially. Localization adds scalar accumulators only.
    let residual_peak_bytes = checked_sum(&[
        checked_mul(source_state_bytes, 3)?,
        integration_force.storage_bytes,
        residual_force_bytes,
        conservative_workspace_bytes,
        fft_catalog_bytes,
        checked_mul(diagnostic_field_bytes, 10)?,
        FIXED_OVERHEAD_BYTES,
    ])?;
    let admitted_peak_bytes = reconstruction_peak_bytes.max(residual_peak_bytes);
    Ok(ReservationOutput {
        schema: "p10-offline-residual-probe-reservation-v1",
        source_state_bytes,
        diagnostic_field_bytes,
        fft_catalog_bytes,
        integration_rhs_bytes,
        integration_force_bytes: integration_force.storage_bytes,
        residual_force_bytes,
        conservative_workspace_bytes,
        reconstruction_peak_bytes,
        residual_peak_bytes,
        admitted_peak_bytes,
        cap_bytes: cap,
    })
}

fn field_bytes(domain: Domain) -> Result<usize, String> {
    domain
        .layout()
        .half_len()
        .checked_mul(3 * std::mem::size_of::<Complex64>())
        .ok_or_else(|| "field byte count overflow".into())
}

fn checked_mul(left: usize, right: usize) -> Result<usize, String> {
    left.checked_mul(right)
        .ok_or_else(|| "reservation multiplication overflow".into())
}

fn checked_sum(values: &[usize]) -> Result<usize, String> {
    values.iter().try_fold(0_usize, |total, &value| {
        total
            .checked_add(value)
            .ok_or_else(|| "reservation sum overflow".into())
    })
}

#[derive(Debug)]
struct Node {
    value: Snapshot,
    derivative: Field,
}

#[derive(Debug)]
struct Reconstruction {
    value: Field,
    derivative: Field,
}

#[derive(Debug)]
struct LocalizedResidualResult {
    localization: nsbu_solver::diagnostics::residual::ResidualLocalization,
    coefficients: Field,
    base_force_work: nsbu_solver::integrators::forcing::ForceWork,
    control_force_work: nsbu_solver::integrators::forcing::ForceWork,
}

fn evaluate(
    plan: &ProbePlan,
    snapshot_root: PathBuf,
    reservations: ReservationOutput,
) -> Result<LocalizationRunOutput, String> {
    let source = plan.domain()?;
    let mut nodes = Vec::new();
    let support = SUPPORTS[2];
    eprintln!("offline-probe: reconstructing fine support={support:?}");
    let mut rhs = build_w3_rhs(
        source,
        Layout::new(plan.integration_force_dimensions).map_err(debug)?,
        plan.integration_force_workers,
        plan.advective_limit,
    )
    .map_err(debug)?;
    let left = load_node(
        plan,
        binding(plan, support[0])?,
        &snapshot_root,
        &mut rhs,
        &mut nodes,
    )?;
    let middle = load_node(
        plan,
        binding(plan, support[1])?,
        &snapshot_root,
        &mut rhs,
        &mut nodes,
    )?;
    let right = load_node(
        plan,
        binding(plan, support[2])?,
        &snapshot_root,
        &mut rhs,
        &mut nodes,
    )?;
    let reconstruction = reconstruct(source, support, [&left, &middle, &right])?;
    drop(rhs);
    drop(left);
    drop(middle);
    drop(right);
    let reconstructed_value_sha256 = hash_field(&reconstruction.value);
    let reconstructed_derivative_sha256 = hash_field(&reconstruction.derivative);
    if reconstructed_value_sha256
        != "10ee2d2fa114620628e3a9b142881e6cc4594348bcf79beb933e14cfa2bae865"
        || reconstructed_derivative_sha256
            != "7fb0c8b34cab4b773df75f4947c6c209369762f624ad4ed3639feb595bcb522c"
    {
        return Err("fine reconstruction replay hash mismatch".into());
    }
    let residual = localized_residual(plan, source, &reconstruction)?;
    let base_residual_sha256 = hash_field(&residual.coefficients);
    if base_residual_sha256 != "0f156b5c1ca4470a34c0a1524601a7bc12e53ad9e86781c33cb48fe07d7bd9b8" {
        return Err("base M768 residual replay hash mismatch".into());
    }
    if residual.localization.retained_modes != 28_164_288
        || residual.localization.new_shell_modes != 197_738_688
        || residual.localization.excluded_nyquist_slots != 1_179_264
    {
        return Err("strict retained/shell partition mismatch".into());
    }
    let identity_closure = enforce_identity_tolerance(&residual.localization, 5e-11)?;
    nodes.sort_by_key(|node| node.clock);
    Ok(LocalizationRunOutput {
        schema: "p10-offline-residual-localization-result-v1",
        status: "one_probe_fine_localization_complete_diagnostic_only",
        source_commit: SOURCE,
        w3_source_commit: W3_SOURCE,
        case_sha256: CASE_SHA256,
        frozen_plan_sha256: PLAN_SHA256,
        profile: PROFILE,
        arithmetic: "binary64; empirical values have no outward-rounding or interval enclosure",
        integration_force: "exact archived constructor: RustFft6_4_1AvxFma FftCatalog + layout576 width3 bidirectional rotational RHS add9200779136 + cached parallel-reduced layout384 width3 forward provider add1827942144 with32 workers",
        base_residual_force: "fresh independent AVX scalar parallel-reduced M768/32-worker provider at clock1112",
        discrete_retained_force_control: "fresh exact integration CachedReducedForce targeting N384/M384 at clock1112; strict zero padding; R384=R768+P(f768-pad(f384)); changes the discrete target equation outside the retained band",
        retained_grid: [384; 3],
        diagnostic_grid: [768; 3],
        probe_clock: PROBE,
        support,
        reservations,
        nodes,
        reconstructed_value_sha256,
        reconstructed_derivative_sha256,
        base_residual_sha256,
        base_force_work_units: residual.base_force_work.work_units,
        base_force_scalar_transforms: residual.base_force_work.scalar_transforms,
        control_force_work_units: residual.control_force_work.work_units,
        control_force_scalar_transforms: residual.control_force_work.scalar_transforms,
        localization: residual.localization.into(),
        identity_closure,
        runtime_owner_imported: false,
        accepted_interpolation: false,
        acceptance_windows: 0,
        qualification: "offline empirical binary64 localization only; no velocity-budget, residual-pass, qualification, accepted interpolation, or accepted-window claim",
    })
}

fn enforce_identity_tolerance(
    localization: &nsbu_solver::diagnostics::residual::ResidualLocalization,
    residual_relative_tolerance: f64,
) -> Result<IdentityClosureOutput, String> {
    let processed_modes = localization
        .retained_modes
        .checked_add(localization.new_shell_modes)
        .ok_or("identity work count overflow")?;
    let term_scaled_tolerance = 64.0 * processed_modes as f64 * f64::EPSILON;
    let mut residual_relative_maximum = 0.0_f64;
    let mut term_scaled_maximum = 0.0_f64;
    for (band_name, band) in [
        ("retained_strict_n384", localization.retained_strict_n384),
        ("new_shell_n768", localization.new_shell_n768),
        ("full_n768", localization.full_n768),
    ] {
        let base = observe_identity(
            band_name,
            "base_m768",
            [band.derivative, band.viscous, band.conservative_m768],
            band.residual_m768,
            band.base_cross,
            residual_relative_tolerance,
            term_scaled_tolerance,
        );
        residual_relative_maximum = residual_relative_maximum.max(base.residual_relative_error);
        term_scaled_maximum = term_scaled_maximum.max(base.term_scaled_error);
        let control = observe_identity(
            band_name,
            "discrete_retained_force_m384",
            [band.derivative, band.viscous, band.conservative_m384],
            band.residual_m384,
            band.control_cross,
            residual_relative_tolerance,
            term_scaled_tolerance,
        );
        residual_relative_maximum = residual_relative_maximum.max(control.residual_relative_error);
        term_scaled_maximum = term_scaled_maximum.max(control.term_scaled_error);
    }
    let output = IdentityClosureOutput {
        residual_relative_tolerance,
        residual_relative_maximum,
        residual_relative_passed: residual_relative_maximum <= residual_relative_tolerance,
        term_scaled_tolerance,
        term_scaled_maximum,
        term_scaled_passed: term_scaled_maximum <= term_scaled_tolerance,
        term_scaled_bound_basis: "64 rounded accumulator contributions per processed mode times binary64 epsilon; residual-relative 5e-11 status is reported independently and is not relabeled",
    };
    eprintln!(
        "identity-gate residual_relative_maximum={:.17e} residual_relative_tolerance={:.17e} residual_relative_passed={} term_scaled_maximum={:.17e} term_scaled_tolerance={:.17e} term_scaled_passed={}",
        output.residual_relative_maximum,
        output.residual_relative_tolerance,
        output.residual_relative_passed,
        output.term_scaled_maximum,
        output.term_scaled_tolerance,
        output.term_scaled_passed
    );
    if !output.term_scaled_passed {
        return Err("term-scaled cross-term norm identity exceeded floating-point bound".into());
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug)]
struct IdentityObservation {
    residual_relative_error: f64,
    term_scaled_error: f64,
}

fn observe_identity(
    band: &str,
    equation: &str,
    terms: [nsbu_solver::diagnostics::norms::Norms; 3],
    residual: nsbu_solver::diagnostics::norms::Norms,
    cross: [nsbu_solver::diagnostics::norms::SignedNormChannels; 3],
    residual_relative_tolerance: f64,
    term_scaled_tolerance: f64,
) -> IdentityObservation {
    let channels = [
        (
            "l2",
            terms.map(|term| term.l2),
            residual.l2,
            cross.map(|term| term.l2),
        ),
        (
            "h1",
            terms.map(|term| term.h1),
            residual.h1,
            cross.map(|term| term.h1),
        ),
        (
            "vorticity_l2",
            terms.map(|term| term.vorticity_l2),
            residual.vorticity_l2,
            cross.map(|term| term.vorticity_l2),
        ),
        (
            "divergence_l2",
            terms.map(|term| term.divergence_l2),
            residual.divergence_l2,
            cross.map(|term| term.divergence_l2),
        ),
    ];
    let mut maximum = IdentityObservation {
        residual_relative_error: 0.0,
        term_scaled_error: 0.0,
    };
    for (channel, terms, residual, cross) in channels {
        let observation = identity_scalars(terms, residual, cross);
        eprintln!(
            "identity-observation band={band} equation={equation} channel={channel} lhs_residual_squared={:.17e} rhs_signed_terms_cross={:.17e} absolute_discrepancy={:.17e} sum_absolute_contributions={:.17e} residual_relative_denominator={:.17e} residual_relative_error={:.17e} residual_relative_tolerance={residual_relative_tolerance:.17e} residual_relative_passed={} term_scaled_denominator={:.17e} term_scaled_error={:.17e} term_scaled_tolerance={term_scaled_tolerance:.17e} term_scaled_passed={}",
            observation.lhs,
            observation.rhs,
            observation.absolute_discrepancy,
            observation.sum_absolute_contributions,
            observation.residual_relative_denominator,
            observation.residual_relative_error,
            observation.residual_relative_error <= residual_relative_tolerance,
            observation.term_scaled_denominator,
            observation.term_scaled_error,
            observation.term_scaled_error <= term_scaled_tolerance,
        );
        maximum.residual_relative_error = maximum
            .residual_relative_error
            .max(observation.residual_relative_error);
        maximum.term_scaled_error = maximum.term_scaled_error.max(observation.term_scaled_error);
    }
    maximum
}

#[derive(Clone, Copy, Debug)]
struct ScalarIdentityObservation {
    lhs: f64,
    rhs: f64,
    absolute_discrepancy: f64,
    sum_absolute_contributions: f64,
    residual_relative_denominator: f64,
    residual_relative_error: f64,
    term_scaled_denominator: f64,
    term_scaled_error: f64,
}

fn identity_scalars(terms: [f64; 3], residual: f64, cross: [f64; 3]) -> ScalarIdentityObservation {
    let term_squares = terms.map(|term| term * term);
    let lhs = residual * residual;
    let rhs = term_squares.into_iter().sum::<f64>() + cross.into_iter().sum::<f64>();
    let absolute_discrepancy = (lhs - rhs).abs();
    let sum_absolute_contributions =
        term_squares.into_iter().sum::<f64>() + cross.into_iter().map(f64::abs).sum::<f64>();
    let residual_relative_denominator = lhs.abs().max(rhs.abs()).max(1.0);
    let term_scaled_denominator = sum_absolute_contributions.max(lhs.abs()).max(1.0);
    ScalarIdentityObservation {
        lhs,
        rhs,
        absolute_discrepancy,
        sum_absolute_contributions,
        residual_relative_denominator,
        residual_relative_error: absolute_discrepancy / residual_relative_denominator,
        term_scaled_denominator,
        term_scaled_error: absolute_discrepancy / term_scaled_denominator,
    }
}

fn binding(plan: &ProbePlan, wanted: u128) -> Result<&NodeBinding, String> {
    plan.nodes
        .iter()
        .find(|node| node.clock == wanted)
        .ok_or_else(|| format!("missing clock {wanted} binding"))
}

fn load_node(
    plan: &ProbePlan,
    binding: &NodeBinding,
    root: &std::path::Path,
    rhs: &mut SpectralRhs<CachedReducedForce>,
    ledger: &mut Vec<NodeOutput>,
) -> Result<Node, String> {
    eprintln!("offline-probe: hash-decoding clock={}", binding.clock);
    let value = decode::load(plan, binding, root)?;
    let mut derivative = field(plan.domain()?)?;
    let node_clock = clock(binding.clock);
    rhs.begin_attempt(node_clock, 64).map_err(debug)?;
    rhs.evaluate(
        value.coefficients.each_ref().map(Vec::as_slice),
        node_clock,
        derivative.each_mut().map(Vec::as_mut_slice),
    )
    .map_err(debug)?;
    add_viscosity(plan.domain()?, &value.coefficients, &mut derivative).map_err(debug)?;
    let consumption = rhs.consumption();
    let hit_miss = rhs.provider().hit_miss();
    ledger.push(NodeOutput {
        clock: binding.clock,
        state_file_sha256: binding.file_sha256.clone(),
        coefficient_sha256: binding.coefficient_sha256.clone(),
        physical_derivative_sha256: hash_field(&derivative),
        rhs_calls: consumption[0],
        rhs_work_units: consumption[1],
        rhs_scalar_transforms: consumption[2],
        force_cache_hits: hit_miss[0],
        force_cache_misses: hit_miss[1],
    });
    Ok(Node { value, derivative })
}

fn build_w3_rhs(
    domain: Domain,
    samples: Layout,
    workers: usize,
    advective_limit: f64,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let catalog = FftCatalog::new(backend, catalog_bytes)?;
    let limits = CachedReducedForce::preflight(domain, samples, workers, backend, true)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        workers,
        &catalog,
        limits.storage_bytes,
        true,
    )?;
    let bytes = SpectralRhs::<CachedReducedForce>::reservation_with_w3_fft_backend(
        domain, limits, backend,
    )?;
    let rhs = SpectralRhs::new_with_catalog_w3(domain, force, advective_limit, &catalog, bytes)?;
    require_w3_identity(&rhs, domain, samples, backend)?;
    Ok(rhs)
}

fn require_w3_identity(
    rhs: &SpectralRhs<CachedReducedForce>,
    domain: Domain,
    samples: Layout,
    backend: FftBackend,
) -> Result<(), SolverError> {
    let expected_rhs = W3FftIdentity {
        layout: domain.padded_layout()?,
        backend,
        width: 3,
        mode: W3FftMode::Bidirectional,
        additional_bytes: W3FftPool::additional_reservation_with_backend(
            domain.padded_layout()?,
            backend,
            W3FftMode::Bidirectional,
        )?,
    };
    let expected_force = W3FftIdentity {
        layout: samples,
        backend,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: W3FftPool::additional_reservation_with_backend(
            samples,
            backend,
            W3FftMode::Forward,
        )?,
    };
    if rhs.w3_fft_identity() != Some(expected_rhs)
        || rhs.provider().w3_identity() != Some(expected_force)
    {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

fn add_viscosity(source: Domain, velocity: &Field, output: &mut Field) -> Result<(), SolverError> {
    for axis in 0..3 {
        for (index, value) in output[axis].iter_mut().enumerate() {
            let position = source.layout().position(index)?;
            if source.layout().is_nyquist(position)? {
                *value = Complex64::new(0.0, 0.0);
                continue;
            }
            let wave = modal::wavevector(source, source.layout().mode(position)?)?;
            let squared = wave
                .into_iter()
                .map(|component| component * component)
                .sum::<f64>();
            *value -= source.viscosity() * squared * velocity[axis][index];
            if !value.re.is_finite() || !value.im.is_finite() {
                return Err(SolverError::InvalidSpectrum);
            }
        }
    }
    Ok(())
}

fn reconstruct(
    source: Domain,
    support: [u128; 3],
    nodes: [&Node; 3],
) -> Result<Reconstruction, String> {
    let weights = HermiteWeights::at(support.map(clock), clock(PROBE)).map_err(debug)?;
    let mut value = field(source)?;
    let mut derivative = field(source)?;
    for axis in 0..3 {
        weights
            .apply(
                [
                    &nodes[0].value.coefficients[axis],
                    &nodes[1].value.coefficients[axis],
                    &nodes[2].value.coefficients[axis],
                    &nodes[0].derivative[axis],
                    &nodes[1].derivative[axis],
                    &nodes[2].derivative[axis],
                ],
                &mut value[axis],
                &mut derivative[axis],
            )
            .map_err(debug)?;
    }
    Ok(Reconstruction { value, derivative })
}

fn localized_residual(
    plan: &ProbePlan,
    source: Domain,
    reconstruction: &Reconstruction,
) -> Result<LocalizedResidualResult, String> {
    eprintln!("offline-probe: M384 force control and independent N768 residual at clock={PROBE}");
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).map_err(debug)?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend).map_err(debug)?;
    let catalog = FftCatalog::new(backend, catalog_bytes).map_err(debug)?;
    let retained_samples = Layout::new(plan.integration_force_dimensions).map_err(debug)?;
    let retained_limits = CachedReducedForce::preflight(
        source,
        retained_samples,
        plan.integration_force_workers,
        backend,
        true,
    )
    .map_err(debug)?;
    let mut retained_provider = CachedReducedForce::new(
        source,
        retained_samples,
        plan.integration_force_workers,
        &catalog,
        retained_limits.storage_bytes,
        true,
    )
    .map_err(debug)?;
    let expected_retained = W3FftIdentity {
        layout: retained_samples,
        backend,
        width: 3,
        mode: W3FftMode::Forward,
        additional_bytes: 1_827_942_144,
    };
    if retained_provider.w3_identity() != Some(expected_retained) {
        return Err("retained force constructor identity mismatch".into());
    }
    let mut retained_force = field(source)?;
    retained_provider
        .begin_attempt(clock(PROBE), 64, retained_limits)
        .map_err(debug)?;
    let control_force_work = retained_provider
        .evaluate(
            clock(PROBE),
            retained_limits,
            retained_force.each_mut().map(Vec::as_mut_slice),
        )
        .map_err(debug)?;
    if retained_provider.hit_miss() != [0, 1] {
        return Err("retained force cache accounting mismatch".into());
    }
    drop(retained_provider);
    let samples = Layout::new(plan.residual_force_dimensions).map_err(debug)?;
    let limits = ParallelReducedV2Force::preflight_with_fft_backend(
        diagnostic,
        samples,
        plan.residual_force_workers,
        backend,
    )
    .map_err(debug)?;
    let mut provider = ParallelReducedV2Force::new_with_catalog(
        diagnostic,
        samples,
        plan.residual_force_workers,
        &catalog,
        limits.storage_bytes,
    )
    .map_err(debug)?;
    let mut force = field(diagnostic)?;
    let base_force_work = provider
        .evaluate(
            clock(PROBE),
            limits,
            force.each_mut().map(Vec::as_mut_slice),
        )
        .map_err(debug)?;
    drop(provider);
    let conservative_bytes =
        ConservativeWorkspace::reservation_with_catalog(source, &catalog).map_err(debug)?;
    let mut products =
        ConservativeWorkspace::new_with_catalog(source, &catalog, conservative_bytes)
            .map_err(debug)?;
    let mut conservative = field(diagnostic)?;
    let mut pressure = filled(diagnostic.layout().half_len())?;
    products
        .evaluate(
            reconstruction.value.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            conservative.each_mut().map(Vec::as_mut_slice),
            &mut pressure,
        )
        .map_err(debug)?;
    drop(pressure);
    drop(products);
    let mut residual = field(diagnostic)?;
    let localization = ResidualPlan::new(source)
        .map_err(debug)?
        .evaluate_localized(
            reconstruction.value.each_ref().map(Vec::as_slice),
            reconstruction.derivative.each_ref().map(Vec::as_slice),
            conservative.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            retained_force.each_ref().map(Vec::as_slice),
            residual.each_mut().map(Vec::as_mut_slice),
        )
        .map_err(debug)?;
    Ok(LocalizedResidualResult {
        localization,
        coefficients: residual,
        base_force_work,
        control_force_work,
    })
}

#[cfg(test)]
fn build_scalar_rhs_reference(
    domain: Domain,
    samples: Layout,
    workers: usize,
    advective_limit: f64,
) -> Result<SpectralRhs<CachedReducedForce>, SolverError> {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    backend.ensure_available()?;
    let catalog_bytes = FftCatalog::reservation(backend)?;
    let catalog = FftCatalog::new(backend, catalog_bytes)?;
    let limits = CachedReducedForce::preflight(domain, samples, workers, backend, false)?;
    let force = CachedReducedForce::new(
        domain,
        samples,
        workers,
        &catalog,
        limits.storage_bytes,
        false,
    )?;
    let bytes =
        SpectralRhs::<CachedReducedForce>::reservation_with_fft_backend(domain, limits, backend)?;
    SpectralRhs::new_with_catalog(domain, force, advective_limit, &catalog, bytes)
}

fn field(domain: Domain) -> Result<Field, String> {
    Ok([
        filled(domain.layout().half_len())?,
        filled(domain.layout().half_len())?,
        filled(domain.layout().half_len())?,
    ])
}

fn filled(length: usize) -> Result<Vec<Complex64>, String> {
    let mut values = Vec::new();
    values.try_reserve_exact(length).map_err(debug)?;
    values.resize(length, Complex64::new(0.0, 0.0));
    Ok(values)
}

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).expect("frozen valid clock")
}

fn hash_field(field: &Field) -> String {
    let mut hash = Sha256::new();
    for component in field {
        for value in component {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests;
