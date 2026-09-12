use nsbu_benchmarks::provider::{
    parallel::ParallelV2Force, parallel_reduced::ParallelReducedV2Force,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{
        validate_spectrum, Domain, Epoch, ExtraStorage, Layout, ResourcePlan, SpectralState,
        TickClock,
    },
    integrators::{
        attempt::AttemptWorkspace,
        forcing::{ForceLimits, PrescribedForce},
        indicator::Tolerances,
        method::Method,
        rhs::SpectralRhs,
        transaction::{commit_candidate, CandidateState},
    },
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

const CAP: usize = 8 * 1024 * 1024 * 1024;
const N: usize = 64;
const M: usize = 192;
const WORKERS: usize = 32;
const STEP: u128 = 128;

struct Prepared {
    limits: ForceLimits,
    rhs_bytes: usize,
    attempt_bytes: usize,
    plan: ResourcePlan,
}

fn main() -> Result<(), SolverError> {
    let dry = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--dry-run") => true,
        _ => return Err(SolverError::InvalidPayload),
    };
    let domain = Domain::new([N; 3], [1.0; 3], 1.0)?;
    let samples = Layout::new([M; 3])?;
    let original_limits = ParallelV2Force::preflight(domain, samples, WORKERS)?;
    let reduced_limits = ParallelReducedV2Force::preflight(domain, samples, WORKERS)?;
    let original = prepare::<ParallelV2Force>(domain, original_limits)?;
    let reduced = prepare::<ParallelReducedV2Force>(domain, reduced_limits)?;
    let outputs = domain
        .layout()
        .half_len()
        .checked_mul(16 * 3 * 2)
        .ok_or(SolverError::SizeOverflow)?;
    let peak = original
        .plan
        .total()
        .max(reduced.plan.total())
        .checked_add(outputs)
        .and_then(|value| value.checked_add(65_536))
        .ok_or(SolverError::SizeOverflow)?;
    println!(
        "preflight source={} retained={N} sampled={M} workers={WORKERS} quantum=-20 target=8192 from_rest=true step={STEP} method=CoxMatthews cap={CAP} sequential_owner_peak_bytes={peak} original_force={original_limits:?} original_rhs_bytes={} original_attempt_bytes={} original_plan_bytes={} reduced_force={reduced_limits:?} reduced_rhs_bytes={} reduced_attempt_bytes={} reduced_plan_bytes={}",
        env!("RUN_SOURCE"),
        original.rhs_bytes,
        original.attempt_bytes,
        original.plan.total(),
        reduced.rhs_bytes,
        reduced.attempt_bytes,
        reduced.plan.total(),
    );
    if peak > CAP {
        return Err(SolverError::ResourceLimit);
    }
    if dry {
        return Ok(());
    }

    let original_provider =
        ParallelV2Force::new(domain, samples, WORKERS, original_limits.storage_bytes)?;
    let original_output = run("original_parallel", domain, original, original_provider)?;
    let reduced_provider =
        ParallelReducedV2Force::new(domain, samples, WORKERS, reduced_limits.storage_bytes)?;
    println!("reduced_identity={:?}", reduced_provider.identity());
    let reduced_output = run("reduced_parallel", domain, reduced, reduced_provider)?;

    let report = ComparisonPlan::new(domain, domain)?.compare(
        original_output.each_ref().map(Vec::as_slice),
        reduced_output.each_ref().map(Vec::as_slice),
    )?;
    let maximum_scaled = original_output
        .iter()
        .flatten()
        .zip(reduced_output.iter().flatten())
        .fold(0.0_f64, |maximum, (left, right)| {
            maximum.max((*left - *right).l1_norm() / (1.0 + left.l1_norm()))
        });
    println!(
        "terminal=complete independently_evolved=true accepted_steps=1 endpoint=128 original_reduced_comparison={report:?} maximum_scaled_coefficient_difference={maximum_scaled:e} arithmetic_identity=separate accepted_pde_windows=0"
    );
    Ok(())
}

fn prepare<F: PrescribedForce>(
    domain: Domain,
    limits: ForceLimits,
) -> Result<Prepared, SolverError> {
    let rhs_bytes = SpectralRhs::<F>::reservation(domain, limits)?;
    let attempt_bytes = AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews)?;
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: rhs_bytes,
            diagnostics: attempt_bytes,
            overhead: 1024 * 1024,
        },
        CAP,
        Epoch(0),
    )?;
    Ok(Prepared {
        limits,
        rhs_bytes,
        attempt_bytes,
        plan,
    })
}

fn run<F: PrescribedForce>(
    label: &str,
    domain: Domain,
    prepared: Prepared,
    provider: F,
) -> Result<[Vec<Complex64>; 3], SolverError> {
    let clock = TickClock::from_rest(-20, 8192)?;
    let mut state = SpectralState::from_rest(prepared.plan, clock, Epoch(0))?;
    let mut candidate = CandidateState::new(prepared.plan, clock, Epoch(0))?;
    let mut workspace = AttemptWorkspace::new_with_method(prepared.plan, Method::CoxMatthews)?;
    let mut rhs = SpectralRhs::new(domain, provider, 0.3, prepared.rhs_bytes)?;
    let started = Instant::now();
    let outcome = workspace.try_advance(
        &state,
        &mut candidate,
        STEP,
        Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [1e-5; 2],
        },
        &mut rhs,
    )?;
    let outcome_debug = format!("{outcome:?}");
    let token = outcome
        .accepted
        .ok_or(SolverError::ArithmeticResolutionLimited)?;
    commit_candidate(state.plan(), &mut state, &mut candidate, token)?;
    let seconds = started.elapsed().as_secs_f64();
    let output = std::array::from_fn(|axis| state.component(axis).unwrap().to_vec());
    for component in &output {
        if component.iter().any(|value| !value.is_finite()) {
            return Err(SolverError::InvalidSpectrum);
        }
        validate_spectrum(domain.layout(), component, 1e-12)?;
    }
    println!(
        "trajectory={label} seconds={seconds:.9} outcome={outcome_debug} clock={} accepted_steps={} rhs_consumption={:?} force_limits={:?} sha256={}",
        state.clock().elapsed(),
        state.accepted_steps(),
        rhs.consumption(),
        prepared.limits,
        fingerprint(&output),
    );
    Ok(output)
}

fn fingerprint(values: &[Vec<Complex64>; 3]) -> String {
    let mut hash = Sha256::new();
    for value in values.iter().flatten() {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}
