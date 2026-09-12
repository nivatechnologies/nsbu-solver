use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::{AttemptResult, AttemptWorkspace},
        indicator::Tolerances,
        kernel::{RhsBounds, RightHandSide},
        method::Method,
        transaction::CandidateState,
    },
    Complex64, SolverError,
};
use sha2::{Digest, Sha256};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{
    alloc::System,
    process::ExitCode,
    time::{Duration, Instant},
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
const BASE_SOURCE: &str = "ba2ad3904769dad0f8611daed307d352d8ec5d6a";

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal=error detail={error}");
            ExitCode::from(1)
        }
    }
}

fn execute() -> Result<(), String> {
    let command = std::env::args().nth(1).unwrap_or_else(|| "validate".into());
    if std::env::args().nth(2).is_some() {
        return Err("usage: harness validate|profile-384".into());
    }
    println!("identity base_source={BASE_SOURCE} method=cox-matthews rhs=zero cache_key=dt_bits+half_dt_bits baseline_attempt_workspace_bytes=784 cached_attempt_workspace_bytes={}", std::mem::size_of::<AttemptWorkspace>());
    match command.as_str() {
        "validate" => profile(4, false),
        "profile-384" => profile(384, true),
        _ => Err("usage: harness validate|profile-384".into()),
    }
}

fn profile(n: usize, apply_gate: bool) -> Result<(), String> {
    let miss = run_owner(n, 4, 8)?;
    let hit = run_owner(n, 8, 8)?;
    if miss.hash != hit.hash || miss.indicators != hit.indicators || miss.rhs_calls != hit.rhs_calls
    {
        return Err("same target dt/state hit and miss outputs differ".into());
    }
    print("forced_miss", &miss);
    print("cache_hit", &hit);
    let saved = miss.outside_seconds - hit.outside_seconds;
    let fraction = saved / miss.outside_seconds;
    let seconds_pass = saved >= 10.0;
    let fraction_pass = fraction >= 0.20;
    println!("gate n={n} miss_minus_hit_seconds={saved:.9} outside_fraction_saved={fraction:.9} seconds_pass={seconds_pass} fraction_pass={fraction_pass} exact=true zero_steady_allocations=true");
    if apply_gate && !(seconds_pass && fraction_pass) {
        println!("decision=stop-negative-result");
    }
    println!("terminal=profile-complete");
    Ok(())
}

fn run_owner(n: usize, warm_ticks: u128, target_ticks: u128) -> Result<Record, String> {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0).map_err(debug)?;
    let diagnostics =
        AttemptWorkspace::reservation_with_method(domain, Method::CoxMatthews).map_err(debug)?;
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics,
            overhead: 4096,
        },
        128 * 1024 * 1024 * 1024,
        Epoch(0),
    )
    .map_err(debug)?;
    let clock = TickClock::from_rest(-10, 1024).map_err(debug)?;
    let state = SpectralState::from_rest(plan, clock, Epoch(0)).map_err(debug)?;
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).map_err(debug)?;
    let mut workspace =
        AttemptWorkspace::new_with_method(plan, Method::CoxMatthews).map_err(debug)?;
    let mut rhs = ZeroRhs::default();
    attempt(&mut workspace, &state, &mut candidate, warm_ticks, &mut rhs)?;
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let result = attempt(
        &mut workspace,
        &state,
        &mut candidate,
        target_ticks,
        &mut rhs,
    )?;
    let wall = started.elapsed();
    let stats = region.change();
    no_allocations(stats)?;
    let token = result.accepted.ok_or("zero RHS target rejected")?;
    let proposal = candidate.proposal(&state, &token).map_err(debug)?;
    Ok(Record {
        n,
        wall_seconds: wall.as_secs_f64(),
        rhs_seconds: rhs.elapsed.as_secs_f64(),
        outside_seconds: wall.saturating_sub(rhs.elapsed).as_secs_f64(),
        rhs_calls: result.rhs_calls,
        indicators: result.indicators.ratios.map(f64::to_bits),
        hash: hash(proposal),
        stats,
        diagnostics,
    })
}

fn attempt(
    workspace: &mut AttemptWorkspace,
    state: &SpectralState,
    candidate: &mut CandidateState,
    ticks: u128,
    rhs: &mut ZeroRhs,
) -> Result<AttemptResult, String> {
    workspace
        .try_advance(
            state,
            candidate,
            ticks,
            Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
            rhs,
        )
        .map_err(debug)
}

#[derive(Default)]
struct ZeroRhs {
    elapsed: Duration,
    calls: usize,
}
impl RightHandSide for ZeroRhs {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn begin_attempt(&mut self, _clock: TickClock, _ticks: u128) -> Result<(), SolverError> {
        self.elapsed = Duration::ZERO;
        self.calls = 0;
        Ok(())
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        let started = Instant::now();
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
        }
        self.elapsed += started.elapsed();
        self.calls += 1;
        Ok(())
    }
}

struct Record {
    n: usize,
    wall_seconds: f64,
    rhs_seconds: f64,
    outside_seconds: f64,
    rhs_calls: usize,
    indicators: [u64; 2],
    hash: [u8; 32],
    stats: Stats,
    diagnostics: usize,
}

fn print(kind: &str, record: &Record) {
    println!("measurement kind={kind} n={} wall_seconds={:.9} rhs_seconds={:.9} outside_rhs_seconds={:.9} rhs_calls={} indicators_bits={:016x},{:016x} state_sha256={} diagnostics_reservation_bytes={} allocations={} deallocations={} reallocations={}", record.n, record.wall_seconds, record.rhs_seconds, record.outside_seconds, record.rhs_calls, record.indicators[0], record.indicators[1], hex(record.hash), record.diagnostics, record.stats.allocations, record.stats.deallocations, record.stats.reallocations);
}

fn hash(state: &SpectralState) -> [u8; 32] {
    let mut digest = Sha256::new();
    for axis in 0..3 {
        for value in state.component(axis).expect("axis") {
            digest.update(value.re.to_bits().to_le_bytes());
            digest.update(value.im.to_bits().to_le_bytes());
        }
    }
    digest.finalize().into()
}
fn hex(bytes: [u8; 32]) -> String {
    bytes
        .into_iter()
        .map(|value| format!("{value:02x}"))
        .collect()
}
fn no_allocations(stats: Stats) -> Result<(), String> {
    if (stats.allocations, stats.deallocations, stats.reallocations) == (0, 0, 0) {
        Ok(())
    } else {
        Err(format!("steady allocation: {stats:?}"))
    }
}
fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
