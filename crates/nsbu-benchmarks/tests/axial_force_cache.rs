//! Compare complete cached-provider coefficients with the original uncached point path.
use nsbu_benchmarks::{fields, provider::V2Force, time::BenchmarkTime};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::{transfer, FftPlan},
    Complex64,
};
#[test]
fn all_coefficients_and_actual_plane_work_survive_nonmonotone_calls_and_rest() {
    let source = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let sampled = Layout::new([6, 8, 12]).unwrap();
    let limits = V2Force::preflight(source, sampled).unwrap();
    let mut provider = V2Force::new(source, sampled, limits.storage_bytes).unwrap();
    let (fft, mut scratch) = FftPlan::new(sampled, FftPlan::reservation(sampled).unwrap()).unwrap();
    let mut physical: [Vec<f64>; 3] = std::array::from_fn(|_| vec![0.0; sampled.real_len()]);
    let mut actual: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); source.layout().half_len()]);
    let mut full = vec![Complex64::new(0.0, 0.0); sampled.half_len()];
    let mut expected = actual[0].clone();
    for tick in [1, 4, 2, 1, 0, 1] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
        let time = BenchmarkTime::new(clock).unwrap();
        let report = provider
            .evaluate(clock, limits, actual.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        let (plane_iterations, uncached_iterations) = uncached(sampled, time, &mut physical);
        assert_eq!(provider.last_root_iterations(), plane_iterations);
        assert_eq!(report.work_units, sampled.real_len() + plane_iterations);
        assert!(report.work_units <= limits.work_units);
        if tick != 0 {
            assert!(plane_iterations > 0 && plane_iterations < uncached_iterations);
        } else {
            assert_eq!(plane_iterations, 0);
        }
        for axis in 0..3 {
            fft.forward(&physical[axis], &mut full, &mut scratch)
                .unwrap();
            transfer(sampled, source.layout(), &full, &mut expected).unwrap();
            for (left, right) in actual[axis].iter().zip(&expected) {
                assert_eq!(
                    (left.re.to_bits(), left.im.to_bits()),
                    (right.re.to_bits(), right.im.to_bits())
                );
            }
        }
    }
}

fn uncached(sampled: Layout, time: BenchmarkTime, physical: &mut [Vec<f64>; 3]) -> (usize, usize) {
    let [nx, ny, nz] = sampled.dimensions();
    let mut plane_iterations = 0;
    let mut uncached_iterations = 0;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let point = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    k as f64 / nz as f64,
                ];
                let value = fields::evaluate(point, time).unwrap();
                let iterations = value.root.map_or(0, |r| r.iterations);
                uncached_iterations += iterations;
                if i == 0 && j == 0 {
                    plane_iterations += iterations;
                }
                for (axis, values) in physical.iter_mut().enumerate() {
                    values[(i * ny + j) * nz + k] = value.force[axis];
                }
            }
        }
    }
    (plane_iterations, uncached_iterations)
}
