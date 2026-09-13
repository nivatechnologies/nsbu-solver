#[path = "../../../avx-w3-n256-integration-20260912/harness/src/cache.rs"]
mod cache;
mod decode;
mod model;

use cache::CachedReducedForce;
use model::{
    debug, DifferenceOutput, NodeBinding, NodeOutput, NormOutput, ProbePlan, ReservationOutput,
    RunOutput, ScaleOutput, Snapshot, CASE_SHA256, PLAN_SHA256, PROBE, PROFILE, SOURCE, SUPPORTS,
    W3_SOURCE,
};
use nsbu_benchmarks::provider::parallel_reduced::ParallelReducedV2Force;
use nsbu_solver::{
    diagnostics::{
        comparison::ComparisonPlan, conservative::ConservativeWorkspace, hermite::HermiteWeights,
        residual::ResidualPlan,
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
    // Three values plus three derivatives, current and previous reconstructed value/derivative.
    let reconstruction_peak_bytes = checked_sum(&[
        checked_mul(source_state_bytes, 10)?,
        integration_rhs_bytes,
        fft_catalog_bytes,
        FIXED_OVERHEAD_BYTES,
    ])?;
    // Center value/derivative can still be retained at the middle-scale residual. Thirteen
    // diagnostic component fields are force(3), conservative(3), current residual(3),
    // pressure(1), and the previous full residual(3).
    let residual_peak_bytes = checked_sum(&[
        checked_mul(source_state_bytes, 6)?,
        residual_force_bytes,
        conservative_workspace_bytes,
        fft_catalog_bytes,
        checked_mul(diagnostic_field_bytes, 13)?,
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
struct ResidualResult {
    norms: NormOutput,
    coefficients: Field,
    force_work: nsbu_solver::integrators::forcing::ForceWork,
}

fn evaluate(
    plan: &ProbePlan,
    snapshot_root: PathBuf,
    reservations: ReservationOutput,
) -> Result<RunOutput, String> {
    let source = plan.domain()?;
    let mut center: Option<Node> = None;
    let mut previous: Option<(&'static str, Reconstruction)> = None;
    let mut previous_residual: Option<Field> = None;
    let mut nodes = Vec::new();
    let mut scales = Vec::new();
    let mut differences = Vec::new();
    for (index, (label, support)) in ["coarse", "middle", "fine"]
        .into_iter()
        .zip(SUPPORTS)
        .enumerate()
    {
        eprintln!("offline-probe: reconstructing {label} support={support:?}");
        let mut rhs = build_w3_rhs(
            source,
            Layout::new(plan.integration_force_dimensions).map_err(debug)?,
            plan.integration_force_workers,
            plan.advective_limit,
        )
        .map_err(debug)?;
        if center.is_none() {
            center = Some(load_node(
                plan,
                binding(plan, 1152)?,
                &snapshot_root,
                &mut rhs,
                &mut nodes,
            )?);
        }
        let left = load_node(
            plan,
            binding(plan, support[0])?,
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
        let middle = center.as_ref().ok_or("missing shared center")?;
        let current = reconstruct(source, support, [&left, middle, &right])?;
        drop(rhs);
        drop(left);
        drop(right);

        let residual = residual(plan, source, &current)?;
        let value_sha256 = hash_field(&current.value);
        let derivative_sha256 = hash_field(&current.derivative);
        scales.push(ScaleOutput {
            label,
            support,
            probe: PROBE,
            residual_acceleration: residual.norms,
            residual_force_work_units: residual.force_work.work_units,
            residual_force_scalar_transforms: residual.force_work.scalar_transforms,
            residual_sha256: hash_field(&residual.coefficients),
            reconstructed_value_sha256: value_sha256,
            reconstructed_derivative_sha256: derivative_sha256,
        });
        if let (Some((previous_label, earlier)), Some(earlier_residual)) =
            (previous.take(), previous_residual.take())
        {
            differences.push(compare_reconstructions(
                source,
                previous_label,
                &earlier,
                label,
                &current,
                &earlier_residual,
                &residual.coefficients,
            )?);
        }
        previous = Some((label, current));
        previous_residual = Some(residual.coefficients);
        if index == 2 {
            center = None;
        }
    }
    drop(center);
    drop(previous);
    drop(previous_residual);
    nodes.sort_by_key(|node| node.clock);
    Ok(RunOutput {
        schema: "p10-offline-residual-probe-result-v1",
        status: "one_probe_complete_diagnostic_only",
        source_commit: SOURCE,
        w3_source_commit: W3_SOURCE,
        case_sha256: CASE_SHA256,
        frozen_plan_sha256: PLAN_SHA256,
        profile: PROFILE,
        arithmetic: "binary64; empirical values have no outward-rounding or interval enclosure",
        integration_force: "exact archived constructor: RustFft6_4_1AvxFma FftCatalog + layout576 width3 bidirectional rotational RHS add9200779136 + cached parallel-reduced layout384 width3 forward provider add1827942144 with32 workers",
        residual_force: "fresh independent AVX scalar parallel-reduced M768/32-worker provider at clock1112 for each residual",
        retained_grid: [384; 3],
        diagnostic_grid: [768; 3],
        probe_clock: PROBE,
        supports: SUPPORTS,
        support_relationship: "interval widths and spacings are nested; node sets share only clock1152",
        reservations,
        nodes,
        scales,
        differences,
        runtime_owner_imported: false,
        accepted_interpolation: false,
        acceptance_windows: 0,
        qualification: "offline empirical diagnostic; no velocity-budget or pass claim without a reviewed dimensional policy",
    })
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

fn residual(
    plan: &ProbePlan,
    source: Domain,
    reconstruction: &Reconstruction,
) -> Result<ResidualResult, String> {
    eprintln!("offline-probe: independent N768 residual at clock={PROBE}");
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).map_err(debug)?;
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog_bytes = FftCatalog::reservation(backend).map_err(debug)?;
    let catalog = FftCatalog::new(backend, catalog_bytes).map_err(debug)?;
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
    let force_work = provider
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
    drop(force);
    drop(pressure);
    drop(products);
    let mut residual = field(diagnostic)?;
    let norms = ResidualPlan::new(source)
        .map_err(debug)?
        .evaluate(
            reconstruction.value.each_ref().map(Vec::as_slice),
            reconstruction.derivative.each_ref().map(Vec::as_slice),
            conservative.each_ref().map(Vec::as_slice),
            residual.each_mut().map(Vec::as_mut_slice),
        )
        .map_err(debug)?;
    Ok(ResidualResult {
        norms: norms.into(),
        coefficients: residual,
        force_work,
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

fn compare_reconstructions(
    source: Domain,
    left_label: &'static str,
    left: &Reconstruction,
    right_label: &'static str,
    right: &Reconstruction,
    left_residual: &Field,
    right_residual: &Field,
) -> Result<DifferenceOutput, String> {
    let plan = ComparisonPlan::new(source, source).map_err(debug)?;
    let value = plan
        .compare(
            left.value.each_ref().map(Vec::as_slice),
            right.value.each_ref().map(Vec::as_slice),
        )
        .map_err(debug)?;
    let derivative = plan
        .compare(
            left.derivative.each_ref().map(Vec::as_slice),
            right.derivative.each_ref().map(Vec::as_slice),
        )
        .map_err(debug)?;
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).map_err(debug)?;
    let residual = ComparisonPlan::new(diagnostic, diagnostic)
        .map_err(debug)?
        .compare(
            left_residual.each_ref().map(Vec::as_slice),
            right_residual.each_ref().map(Vec::as_slice),
        )
        .map_err(debug)?;
    Ok(DifferenceOutput {
        left: left_label,
        right: right_label,
        reconstructed_velocity: value.full.into(),
        reconstructed_derivative: derivative.full.into(),
        residual_acceleration: residual.full.into(),
    })
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
