//! Shared prescribed smooth-force contract for independent diagnostic sampling.
use nsbu_benchmarks::smooth::CyclicSine;
use nsbu_solver::{domain::TickClock, integrators::forcing::PrescribedForce, Complex64};

pub fn evaluate(evaluator: &mut CyclicSine, clock: TickClock, output: &mut [Vec<Complex64>; 3]) {
    let limits = evaluator.limits().unwrap();
    let [a, b, c] = output;
    let work = evaluator.evaluate(clock, limits, [a, b, c]).unwrap();
    assert_eq!(work.scalar_transforms, 0);
    assert!(work.work_units <= limits.work_units);
}
