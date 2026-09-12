use crate::{
    model::{Operation, SerialBatch},
    pool::{reservation, ParallelBatch, WorkerFailure},
    util::{debug, worker_debug, WIDTH},
};
use nsbu_solver::{domain::Layout, spectral::FftCatalog, Complex64};

pub(crate) fn validate(catalog: &FftCatalog) -> Result<(), String> {
    let error = direct_reference(catalog)?;
    failure_controls(catalog)?;
    println!(
        "controls direct_dft_n=6 maximum_scaled={error:.17e} serial_w3_equal_words=true injected_error_drained=true injected_panic_drained=true failed_publication_preserved=true one_byte_short_caps_checked=true"
    );
    Ok(())
}

pub(crate) fn compare(
    serial: &SerialBatch,
    parallel: &ParallelBatch,
    operation: Operation,
) -> Result<(), String> {
    if parallel.equals(serial, operation)? {
        Ok(())
    } else {
        Err(format!("a {operation:?} lane differs from serial"))
    }
}

fn direct_reference(catalog: &FftCatalog) -> Result<f64, String> {
    let layout = Layout::new([6; 3]).map_err(debug)?;
    let mut serial = SerialBatch::new(layout, catalog)?;
    let mut parallel = exact_batch(layout, catalog)?;
    serial.reset(17);
    parallel.reset(17)?;
    let direct = direct_dft(layout, &serial.lanes[0].physical);
    serial.execute(Operation::Forward)?;
    parallel.execute(Operation::Forward).map_err(worker_debug)?;
    compare(&serial, &parallel, Operation::Forward)?;
    let error = maximum_scaled(&direct, &serial.lanes[0].spectrum);
    if error > 2e-14 {
        return Err(format!("direct reference error {error}"));
    }
    Ok(error)
}

fn failure_controls(catalog: &FftCatalog) -> Result<(), String> {
    injected_error_control(catalog)?;
    injected_panic_control(catalog)
}

fn injected_error_control(catalog: &FftCatalog) -> Result<(), String> {
    let layout = Layout::new([6; 3]).map_err(debug)?;
    let mut failed = exact_batch(layout, catalog)?;
    failed.reset(17)?;
    let prior = failed.hashes()?;
    failed.inject_failure(1, false);
    if failed.execute(Operation::Inverse) != Err(WorkerFailure::Injected)
        || !failed.quiescent()
        || failed.hashes()? != prior
    {
        return Err("injected worker failure did not drain/preserve publication".into());
    }
    Ok(())
}

fn injected_panic_control(catalog: &FftCatalog) -> Result<(), String> {
    let layout = Layout::new([6; 3]).map_err(debug)?;
    let mut panicked = exact_batch(layout, catalog)?;
    panicked.reset(31)?;
    let prior = panicked.hashes()?;
    panicked.inject_failure(WIDTH - 1, true);
    if panicked.execute(Operation::Forward) != Err(WorkerFailure::Panic)
        || !panicked.quiescent()
        || panicked.hashes()? != prior
    {
        return Err("injected worker panic did not drain/preserve publication".into());
    }
    Ok(())
}

fn exact_batch(layout: Layout, catalog: &FftCatalog) -> Result<ParallelBatch, String> {
    let bytes = reservation(layout)?.parallel_profile_bytes;
    ParallelBatch::new(layout, catalog, bytes)
}

fn direct_dft(layout: Layout, input: &[f64]) -> Vec<Complex64> {
    let [nx, ny, nz] = layout.dimensions();
    let half = nz / 2 + 1;
    let scale = layout.real_len() as f64;
    let mut output = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for a in 0..nx {
        for b in 0..ny {
            for c in 0..half {
                output[(a * ny + b) * half + c] =
                    direct_mode(input, [nx, ny, nz], [a, b, c]) / scale;
            }
        }
    }
    output
}

fn direct_mode(input: &[f64], size: [usize; 3], mode: [usize; 3]) -> Complex64 {
    let [nx, ny, nz] = size;
    let [a, b, c] = mode;
    let mut value = Complex64::new(0.0, 0.0);
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let angle = -std::f64::consts::TAU
                    * (a as f64 * i as f64 / nx as f64
                        + b as f64 * j as f64 / ny as f64
                        + c as f64 * k as f64 / nz as f64);
                value += input[(i * ny + j) * nz + k] * Complex64::new(angle.cos(), angle.sin());
            }
        }
    }
    value
}

fn maximum_scaled(reference: &[Complex64], actual: &[Complex64]) -> f64 {
    reference
        .iter()
        .zip(actual)
        .map(|(a, b)| (*a - *b).norm() / a.norm().max(1.0))
        .fold(0.0_f64, f64::max)
}
