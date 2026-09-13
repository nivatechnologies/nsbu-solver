use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan};
use nsbu_solver::Complex64;
use std::fs::{create_dir, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 4, "usage: benchmark N REPEATS OUTPUT_DIR");
    let n = args[1].parse::<usize>().unwrap();
    let repeats = args[2].parse::<usize>().unwrap();
    assert!(repeats > 0);
    let output_dir = Path::new(&args[3]);
    create_dir(output_dir).unwrap();
    let layout = Layout::new([n; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
    let reservation = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
    let (plan, mut work) = FftPlan::new_from_catalog(layout, &catalog, reservation).unwrap();
    let input = (0..layout.real_len())
        .map(|index| {
            let z = index % n;
            let y = index / n % n;
            let x = index / (n * n);
            let phase = std::f64::consts::TAU * (x + 2 * y + 3 * z) as f64 / n as f64;
            phase.cos() + 0.125 * if z % 2 == 0 { 1.0 } else { -1.0 }
        })
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut restored = vec![0.0; layout.real_len()];
    plan.forward(&input, &mut spectrum, &mut work).unwrap();
    plan.inverse(&spectrum, &mut restored, &mut work).unwrap();
    let started = Instant::now();
    for _ in 0..repeats {
        plan.forward(&input, &mut spectrum, &mut work).unwrap();
    }
    let forward_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    for _ in 0..repeats {
        plan.inverse(&spectrum, &mut restored, &mut work).unwrap();
    }
    let inverse_ns = started.elapsed().as_nanos();
    write_complex(&output_dir.join("forward.bin"), &spectrum);
    write_real(&output_dir.join("inverse.bin"), &restored);
    println!(
        "{{\"layout\":{n},\"repeats\":{repeats},\"reservation_bytes\":{reservation},\"forward_ns\":{forward_ns},\"inverse_ns\":{inverse_ns}}}"
    );
}

fn write_complex(path: &Path, values: &[Complex64]) {
    let mut file = BufWriter::new(
        File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap(),
    );
    for value in values {
        file.write_all(&value.re.to_bits().to_le_bytes()).unwrap();
        file.write_all(&value.im.to_bits().to_le_bytes()).unwrap();
    }
}

fn write_real(path: &Path, values: &[f64]) {
    let mut file = BufWriter::new(
        File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap(),
    );
    for value in values {
        file.write_all(&value.to_bits().to_le_bytes()).unwrap();
    }
}
