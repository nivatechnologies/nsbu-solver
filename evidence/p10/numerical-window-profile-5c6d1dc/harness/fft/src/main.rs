use nsbu_solver::{domain::Layout, spectral::FftPlan, Complex64};
use sha2::{Digest, Sha256};
use std::{process::ExitCode, time::Instant};

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
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [n, repeats] = args.as_slice() else {
        return Err("usage: p10-fft-profile N REPEATS".into());
    };
    let n = n.parse::<usize>().map_err(debug)?;
    let repeats = repeats.parse::<usize>().map_err(debug)?;
    if repeats == 0 {
        return Err("repeats must be positive".into());
    }
    let layout = Layout::new([n; 3]).map_err(debug)?;
    let reservation = FftPlan::reservation(layout).map_err(debug)?;
    let io_bytes = layout
        .real_len()
        .checked_mul(8)
        .and_then(|value| value.checked_add(layout.half_len() * 16))
        .ok_or("I/O reservation overflow")?;
    let cap = reservation.checked_add(io_bytes).ok_or("cap overflow")?;
    let (fft, mut work) = FftPlan::new(layout, reservation).map_err(debug)?;
    let mut physical = vec![0.0; layout.real_len()];
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for (index, value) in physical.iter_mut().enumerate() {
        *value = ((index % 31) as f64 - 15.0) / 31.0;
    }
    println!("preflight source={} n={n} repeats={repeats} fft_reservation={reservation} io_bytes={io_bytes} cap={cap}", env!("RUN_SOURCE"));
    let started = Instant::now();
    for _ in 0..repeats {
        fft.forward(&physical, &mut spectrum, &mut work).map_err(debug)?;
        fft.inverse(&spectrum, &mut physical, &mut work).map_err(debug)?;
    }
    let seconds = started.elapsed().as_secs_f64();
    let mut hash = Sha256::new();
    for value in &physical {
        hash.update(value.to_bits().to_le_bytes());
    }
    println!("terminal=complete n={n} repeats={repeats} scalar_transforms={} seconds={seconds:.9} sha256={:x}", repeats * 2, hash.finalize());
    Ok(())
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
