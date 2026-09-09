//! Independent conservative coefficient convolution, energy and physical-pressure tests.
use nsbu_solver::domain::Domain;
use nsbu_solver::spectral::RotationalWorkspace;
use nsbu_solver::Complex64;
use std::collections::BTreeMap;

type Modes = BTreeMap<[isize; 3], [Complex64; 3]>;

fn sparse_profile() -> Modes {
    let c = Complex64::new;
    BTreeMap::from([
        ([0, 0, 0], [c(0.1, 0.0), c(0.2, 0.0), c(-0.3, 0.0)]),
        ([3, 0, 0], [c(0.0, 0.0), c(0.5, 0.2), c(0.0, 0.0)]),
        ([-3, 0, 0], [c(0.0, 0.0), c(0.5, -0.2), c(0.0, 0.0)]),
        ([3, 1, 0], [c(0.0, 0.0), c(0.0, 0.0), c(0.5, -0.1)]),
        ([-3, -1, 0], [c(0.0, 0.0), c(0.0, 0.0), c(0.5, 0.1)]),
    ])
}

fn conservative(profile: &Modes, target: [isize; 3]) -> ([Complex64; 3], Complex64) {
    let mut raw = [Complex64::new(0.0, 0.0); 3];
    for (p, u) in profile {
        for (q, v) in profile {
            if [p[0] + q[0], p[1] + q[1], p[2] + q[2]] != target {
                continue;
            }
            let dot = u[0] * q[0] as f64 + u[1] * q[1] as f64 + u[2] * q[2] as f64;
            for axis in 0..3 {
                raw[axis] -= Complex64::i() * std::f64::consts::TAU * dot * v[axis];
            }
        }
    }
    let k = target.map(|m| std::f64::consts::TAU * m as f64);
    let squared = k.iter().map(|v| v * v).sum::<f64>();
    if squared == 0.0 {
        return (raw, Complex64::new(0.0, 0.0));
    }
    let dot = k[0] * raw[0] + k[1] * raw[1] + k[2] * raw[2];
    let projected = std::array::from_fn(|axis| raw[axis] - k[axis] * dot / squared);
    (projected, -Complex64::i() * dot / squared)
}

#[test]
fn full_retained_band_matches_convolution_and_inviscid_energy_identity() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let profile = sparse_profile();
    let mut velocity =
        std::array::from_fn::<_, 3, _>(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    for (mode, vector) in &profile {
        let index = layout.locate(*mode).unwrap().0;
        for axis in 0..3 {
            velocity[axis][index] = vector[axis];
        }
    }
    let zero = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    let bytes = RotationalWorkspace::reservation(domain).unwrap();
    let mut work = RotationalWorkspace::new(domain, bytes).unwrap();
    let [a, b, c] = &mut output;
    work.evaluate(
        [&velocity[0], &velocity[1], &velocity[2]],
        [&zero; 3],
        [a, b, c],
        &mut pressure,
    )
    .unwrap();
    let mut energy = 0.0;
    for (index, &p) in pressure.iter().enumerate() {
        let position = layout.position(index).unwrap();
        if layout.is_nyquist(position).unwrap() {
            assert_eq!(p, Complex64::new(0.0, 0.0));
            continue;
        }
        let (expected, expected_pressure) = conservative(&profile, layout.mode(position).unwrap());
        assert!(
            (p - expected_pressure).norm_sqr() < 1e-26,
            "pressure at {position:?}"
        );
        for axis in 0..3 {
            assert!(
                (output[axis][index] - expected[axis]).norm_sqr() < 1e-25,
                "axis {axis}, {position:?}"
            );
            energy += layout.weight(position).unwrap()
                * (velocity[axis][index].conj() * output[axis][index]).re;
        }
    }
    assert!(energy.abs() < 1e-13);
    // A six-frequency quadratic interaction must not alias into retained mode (-2,1,0).
    let index = layout.locate([-2, 1, 0]).unwrap().0;
    assert!(output.iter().all(|values| values[index].norm_sqr() < 1e-26));
    assert!(conservative(&profile, [6, 1, 0]).0[2].norm_sqr() > 0.1);
}
