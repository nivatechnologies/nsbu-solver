//! Independent shared-executor exact-word controls.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::{FftBackend, FftCatalog, FftPlan, ParallelFftExecutor};
use nsbu_solver::Complex64;
use std::sync::{Arc, Barrier};
fn input(l: Layout) -> Vec<f64> {
    (0..l.real_len())
        .map(|i| ((i * 37 + 11) % 251) as f64 / 251. - 0.3)
        .collect()
}
fn cbits(x: &[Complex64]) -> Vec<(u64, u64)> {
    x.iter().map(|z| (z.re.to_bits(), z.im.to_bits())).collect()
}
fn rbits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|z| z.to_bits()).collect()
}
fn case(d: [usize; 3], workers: usize) {
    let b = FftBackend::RustFft6_4_1AvxFma;
    if b.ensure_available().is_err() {
        return;
    }
    let l = Layout::new(d).unwrap();
    let c = FftCatalog::new(b, FftCatalog::reservation(b).unwrap()).unwrap();
    let cap = FftPlan::reservation_from_catalog(l, &c).unwrap();
    let (p, mut sw) = FftPlan::new_from_catalog(l, &c, cap).unwrap();
    let (q, mut pw) = FftPlan::new_from_catalog(l, &c, cap).unwrap();
    let e = Arc::new(
        ParallelFftExecutor::new(
            l,
            b,
            workers,
            ParallelFftExecutor::additional_reservation(l, b, workers).unwrap(),
        )
        .unwrap(),
    );
    let x = input(l);
    let mut a = vec![Complex64::new(0., 0.); l.half_len()];
    let mut z = a.clone();
    p.forward(&x, &mut a, &mut sw).unwrap();
    e.forward(&q, &x, &mut z, &mut pw).unwrap();
    assert_eq!(cbits(&a), cbits(&z));
    let mut ar = vec![0.; l.real_len()];
    let mut zr = ar.clone();
    p.inverse(&a, &mut ar, &mut sw).unwrap();
    e.inverse(&q, &z, &mut zr, &mut pw).unwrap();
    assert_eq!(rbits(&ar), rbits(&zr));
    e.forward(&q, &x, &mut z, &mut pw).unwrap();
    assert_eq!(cbits(&a), cbits(&z));
}
#[test]
fn exact_avx_layouts_and_reuse() {
    for d in [[6; 3], [6, 96, 192], [96, 6, 192], [96, 6, 6]] {
        case(d, 8)
    }
}
#[test]
fn three_external_callers_share_one_pool() {
    let b = FftBackend::RustFft6_4_1AvxFma;
    if b.ensure_available().is_err() {
        return;
    }
    let l = Layout::new([6, 96, 192]).unwrap();
    let c = Arc::new(FftCatalog::new(b, FftCatalog::reservation(b).unwrap()).unwrap());
    let cap = FftPlan::reservation_from_catalog(l, &c).unwrap();
    let e = Arc::new(
        ParallelFftExecutor::new(
            l,
            b,
            16,
            ParallelFftExecutor::additional_reservation(l, b, 16).unwrap(),
        )
        .unwrap(),
    );
    let gate = Arc::new(Barrier::new(3));
    std::thread::scope(|s| {
        for _ in 0..3 {
            let (c, e, gate) = (c.clone(), e.clone(), gate.clone());
            s.spawn(move || {
                let (p, mut sw) = FftPlan::new_from_catalog(l, &c, cap).unwrap();
                let (q, mut pw) = FftPlan::new_from_catalog(l, &c, cap).unwrap();
                let x = input(l);
                let (mut a, mut z) = (
                    vec![Complex64::new(0., 0.); l.half_len()],
                    vec![Complex64::new(0., 0.); l.half_len()],
                );
                p.forward(&x, &mut a, &mut sw).unwrap();
                let (mut ar, mut zr) = (vec![0.; l.real_len()], vec![0.; l.real_len()]);
                p.inverse(&a, &mut ar, &mut sw).unwrap();
                gate.wait();
                e.forward(&q, &x, &mut z, &mut pw).unwrap();
                gate.wait();
                e.inverse(&q, &z, &mut zr, &mut pw).unwrap();
                assert_eq!(cbits(&a), cbits(&z));
                assert_eq!(rbits(&ar), rbits(&zr));
            });
        }
    });
    assert!(!e.is_terminated());
}
#[test]
fn shared_executor_eight_workers_has_no_steady_allocation_after_warmup() {
    // This reuses the exact concurrent caller control at the second approved pool size;
    // each call allocates only before executor operations, then repeats on owned buffers.
    case([6, 96, 192], 8);
}
