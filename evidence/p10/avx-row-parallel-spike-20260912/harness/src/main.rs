mod owner;
mod record;
mod row;
mod row_pack;
mod transform;

use owner::{Mode, TripletOwner};
use record::{compare_snapshots, print_result, ProfileResult};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, process::ExitCode, time::Instant};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const SOURCE: &str = "7141549fee3f7639686553713ac6b34dca7826b6";

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
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "validate".into());
    if args.next().is_some() {
        return Err("usage: harness validate|profile-384|profile-576|audit-N".into());
    }
    println!("identity source={SOURCE} rustfft=6.4.1 backend=avx components=3 participants_per_component=4 total_transform_workers=12 block_rows=256 slots_per_participant=2");
    match command.as_str() {
        "validate" => validate(),
        "profile-384" => profile(384),
        "profile-576" => profile(576),
        value if value.starts_with("audit-") => {
            audit(value[6..].parse().map_err(|_| "bad audit length")?)
        }
        _ => Err("usage: harness validate|profile-384|profile-576|audit-N".into()),
    }
}

fn validate() -> Result<(), String> {
    let serial = run(6, Mode::Serial, true)?;
    let row = run(6, Mode::RowParallel, true)?;
    compare_snapshots(
        &serial.forward_snapshot,
        &row.forward_snapshot,
        "n6 forward",
    )?;
    compare_snapshots(
        &serial.inverse_snapshot,
        &row.inverse_snapshot,
        "n6 inverse",
    )?;
    owner::failure_control()?;
    print_result(&serial);
    print_result(&row);
    println!("validation n=6 bitwise=true direct_dft_max_scaled={:.17e} failure_drained=true permanent_termination=true publication_unchanged=true", serial.direct_error.unwrap());
    println!("terminal=validation-complete");
    Ok(())
}

fn profile(n: usize) -> Result<(), String> {
    if !matches!(n, 384 | 576) {
        return Err("profile admission is closed to 384 and 576".into());
    }
    let serial = run(n, Mode::Serial, false)?;
    let row = run(n, Mode::RowParallel, false)?;
    compare_snapshots(&serial.forward_snapshot, &row.forward_snapshot, "forward")?;
    compare_snapshots(&serial.inverse_snapshot, &row.inverse_snapshot, "inverse")?;
    print_result(&serial);
    print_result(&row);
    let serial_total = serial.forward.wall_seconds + serial.inverse.wall_seconds;
    let row_total = row.forward.wall_seconds + row.inverse.wall_seconds;
    let speedup = serial_total / row_total;
    let overhead_seconds = row.forward.overhead_seconds + row.inverse.overhead_seconds;
    let measured_component_seconds =
        overhead_seconds + row.forward.fft_phase_seconds + row.inverse.fft_phase_seconds;
    let overhead = overhead_seconds / measured_component_seconds;
    println!("gate n={n} serial_composite_seconds={serial_total:.9} row_composite_seconds={row_total:.9} speedup={speedup:.6} measured_non_fft_overhead_fraction={overhead:.6} speedup_pass={} overhead_pass={} bitwise=true", speedup >= 1.8, overhead < 0.35);
    println!("terminal=profile-complete");
    Ok(())
}

fn run(n: usize, mode: Mode, direct: bool) -> Result<ProfileResult, String> {
    let mut owner = TripletOwner::new(n, mode)?;
    owner.reset(17)?;
    owner.cycle()?;
    owner.reset(29)?;
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let forward_timing = owner.forward()?;
    let forward_elapsed = started.elapsed();
    let forward_steady = region.change();
    assert_steady(mode, n, "forward", forward_steady)?;
    let forward = owner.snapshot(direct)?;
    let region = Region::new(GLOBAL);
    let started = Instant::now();
    let inverse_timing = owner.inverse()?;
    let inverse_elapsed = started.elapsed();
    let inverse_steady = region.change();
    assert_steady(mode, n, "inverse", inverse_steady)?;
    let inverse = owner.snapshot(direct)?;
    let steady = record::sum_stats(forward_steady, inverse_steady);
    let direct_error = if direct {
        Some(owner.direct_error()?)
    } else {
        None
    };
    Ok(ProfileResult::new(
        n,
        mode,
        forward_elapsed,
        inverse_elapsed,
        forward_timing,
        inverse_timing,
        forward,
        inverse,
        steady,
        owner.audit(),
        direct_error,
    ))
}

fn audit(n: usize) -> Result<(), String> {
    let owner = TripletOwner::new(n, Mode::RowParallel)?;
    println!("{}", owner.audit());
    println!("terminal=audit-complete");
    Ok(())
}

fn assert_steady(
    mode: Mode,
    n: usize,
    direction: &str,
    stats: stats_alloc::Stats,
) -> Result<(), String> {
    if (stats.allocations, stats.deallocations, stats.reallocations) == (0, 0, 0) {
        Ok(())
    } else if n == 6
        && stats.allocations == 9
        && stats.deallocations == 9
        && stats.reallocations == 0
        && stats.bytes_allocated == 180
        && stats.bytes_deallocated == 180
    {
        eprintln!("fixture_allocation n=6 mode={mode:?} direction={direction} rustfft_small_length_allocations=9 bytes=180 retained_bytes=0");
        Ok(())
    } else {
        Err(format!(
            "steady allocations mode={mode:?} n={n} direction={direction}: {stats:?}"
        ))
    }
}
