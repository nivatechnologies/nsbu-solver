use super::*;

pub(super) fn produce_reference(
    bridge: &BridgeManifest,
    preflight: &Preflight,
) -> Result<[Vec<Complex64>; 3], String> {
    let samples = Layout::new(bridge.sample_dimensions).map_err(debug)?;
    let retained = Layout::new(bridge.retained_dimensions).map_err(debug)?;
    let backend = backend(bridge)?;
    let catalog = FftCatalog::new(backend, preflight.storage.fft_catalog).map_err(debug)?;
    let (plan, mut workspace) =
        FftPlan::new_from_catalog(samples, &catalog, preflight.storage.fft_plan_and_workspace)
            .map_err(debug)?;
    eprintln!("{{\"event\":\"reference_allocation_start\"}}");
    let mut physical = three_zeros(samples.real_len())?;
    eprintln!(
        "{{\"event\":\"reference_sampling_start\",\"x_planes\":{}}}",
        samples.dimensions()[0]
    );
    sample_velocity(bridge, &mut physical)?;
    eprintln!("{{\"event\":\"reference_sampling_complete\"}}");
    let mut spectrum = try_zeros(samples.half_len())?;
    let mut result = three_zeros(retained.half_len())?;
    for component in 0..3 {
        eprintln!("{{\"event\":\"reference_fft_start\",\"component\":{component}}}");
        plan.forward(&physical[component], &mut spectrum, &mut workspace)
            .map_err(debug)?;
        transfer(samples, retained, &spectrum, &mut result[component]).map_err(debug)?;
        eprintln!("{{\"event\":\"reference_fft_complete\",\"component\":{component}}}");
    }
    Ok(result)
}

fn sample_velocity(bridge: &BridgeManifest, output: &mut [Vec<f64>; 3]) -> Result<(), String> {
    let clock = TickClock::restore(
        bridge.clock_exponent,
        bridge.clock_target,
        bridge.elapsed,
        bridge.clock_target - bridge.elapsed,
    )
    .map_err(debug)?;
    let time = BenchmarkTime::new(clock).map_err(debug)?;
    let [nx, ny, nz] = bridge.sample_dimensions;
    let started = std::time::Instant::now();
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let index = (i * ny + j) * nz + k;
                let point = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    k as f64 / nz as f64,
                ];
                let velocity = scalar::evaluate(point, time).map_err(debug)?.velocity;
                for component in 0..3 {
                    output[component][index] = velocity[component];
                }
            }
        }
        if i < 8 || (i + 1).is_multiple_of(8) {
            eprintln!(
                "{{\"event\":\"reference_sampling_progress\",\"completed_x_planes\":{},\"elapsed_seconds\":{}}}",
                i + 1,
                started.elapsed().as_secs_f64()
            );
        }
    }
    Ok(())
}

pub(super) fn backend(bridge: &BridgeManifest) -> Result<FftBackend, String> {
    match bridge.fft_backend.as_str() {
        "rustfft-6.4.1-avx-avx2-fma" => Ok(FftBackend::RustFft6_4_1AvxFma),
        "project-owned-mixed-radix-binary64" => Ok(FftBackend::OwnedRadix),
        _ => Err("unsupported FFT backend binding".into()),
    }
}

fn try_zeros<T: Clone + Default>(length: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "bounded allocation failed")?;
    values.resize(length, T::default());
    Ok(values)
}

fn three_zeros<T: Clone + Default>(length: usize) -> Result<[Vec<T>; 3], String> {
    Ok([try_zeros(length)?, try_zeros(length)?, try_zeros(length)?])
}
