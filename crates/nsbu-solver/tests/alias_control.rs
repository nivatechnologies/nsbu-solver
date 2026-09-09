//! Deliberately unpadded sampling must expose the quadratic alias rejected by P03.
use nsbu_solver::Complex64;
use std::f64::consts::TAU;

fn sampled_advective_z(n: usize, target: [isize; 2]) -> Complex64 {
    let mut coefficient = Complex64::new(0.0, 0.0);
    for i in 0..n {
        for j in 0..n {
            let x = i as f64 / n as f64;
            let y = j as f64 / n as f64;
            let a = TAU * 3.0 * x;
            let b = TAU * (3.0 * x + y);
            let uy = 0.2 + a.cos() - 0.4 * a.sin();
            let derivative = TAU * (-b.sin() + 0.2 * b.cos());
            let advection = -(0.3 + uy) * derivative;
            let phase = -TAU * (target[0] as f64 * x + target[1] as f64 * y);
            coefficient += advection * Complex64::new(phase.cos(), phase.sin());
        }
    }
    coefficient / (n * n) as f64
}

#[test]
fn unpadded_product_aliases_while_three_halves_sampling_removes_the_alias() {
    let high = -Complex64::i() * TAU * Complex64::new(0.5, 0.2) * Complex64::new(0.5, -0.1);
    let wrong = sampled_advective_z(8, [-2, 1]);
    assert!((wrong - high).norm_sqr() < 1e-27);
    assert!(wrong.norm_sqr() > 0.1);
    assert!(sampled_advective_z(12, [-2, 1]).norm_sqr() < 1e-27);
}
