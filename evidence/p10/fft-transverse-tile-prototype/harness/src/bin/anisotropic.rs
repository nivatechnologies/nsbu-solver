use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan};
use nsbu_solver::Complex64;
use std::fs::{create_dir, File};
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert!(
        args.len() == 6 || args.len() == 7 && args[6] == "--dense",
        "usage: anisotropic BACKEND NX NY NZ OUTPUT_DIR [--dense]"
    );
    let dimensions = [
        args[2].parse::<usize>().unwrap(),
        args[3].parse::<usize>().unwrap(),
        args[4].parse::<usize>().unwrap(),
    ];
    let layout = Layout::new(dimensions).unwrap();
    let backend = match args[1].as_str() {
        "owned" => FftBackend::OwnedRadix,
        "avx" => FftBackend::RustFft6_4_1AvxFma,
        _ => panic!("unknown backend"),
    };
    let (plan, mut work, reservation) = if backend == FftBackend::OwnedRadix {
        let bytes = FftPlan::reservation(layout).unwrap();
        let (plan, work) = FftPlan::new(layout, bytes).unwrap();
        (plan, work, bytes)
    } else {
        let catalog = FftCatalog::new(backend, FftCatalog::reservation(backend).unwrap()).unwrap();
        let bytes = FftPlan::reservation_from_catalog(layout, &catalog).unwrap();
        let (plan, work) = FftPlan::new_from_catalog(layout, &catalog, bytes).unwrap();
        (plan, work, bytes)
    };
    let [nx, ny, nz] = dimensions;
    let dense = args.len() == 7;
    let input = (0..layout.real_len())
        .map(|index| {
            if dense {
                let bits = (index as u64)
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                return ((bits >> 11) as f64) * 2.0_f64.powi(-53) - 0.5;
            }
            let z = index % nz;
            let y = index / nz % ny;
            let x = index / (ny * nz);
            let phase = std::f64::consts::TAU
                * (x as f64 / nx as f64 + 2.0 * y as f64 / ny as f64 + 3.0 * z as f64 / nz as f64);
            phase.cos() + 0.125 * if z % 2 == 0 { 1.0 } else { -1.0 }
        })
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut restored = vec![0.0; layout.real_len()];
    plan.forward(&input, &mut spectrum, &mut work).unwrap();
    plan.inverse(&spectrum, &mut restored, &mut work).unwrap();
    let output_dir = Path::new(&args[5]);
    create_dir(output_dir).unwrap();
    write_complex(&output_dir.join("forward.bin"), &spectrum);
    write_real(&output_dir.join("inverse.bin"), &restored);
    println!(
        "{{\"dimensions\":{dimensions:?},\"backend\":\"{}\",\"input\":\"{}\",\"reservation_bytes\":{reservation}}}",
        args[1],
        if dense { "dense-lcg-bits-v1" } else { "cosine-plus-nyquist-v1" }
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
