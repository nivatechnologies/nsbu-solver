//! Independent full-complex integer-mode summation; no production modal/weight/norm reducer.
use nsbu_solver::{
    diagnostics::{comparison::BandComparison, norms::Norms},
    domain::Domain,
    Complex64,
};

fn value(n: [usize; 3], fields: [&[Complex64]; 3], mode: [isize; 3]) -> [Complex64; 3] {
    if (0..3).any(|axis| mode[axis].unsigned_abs() >= n[axis] / 2) {
        return [Complex64::new(0.0, 0.0); 3];
    }
    let conjugate = mode[2] < 0;
    let signed = mode.map(|k| if conjugate { -k } else { k });
    let x = signed[0].rem_euclid(n[0] as isize) as usize;
    let y = signed[1].rem_euclid(n[1] as isize) as usize;
    let index = (x * n[1] + y) * (n[2] / 2 + 1) + signed[2] as usize;
    fields.map(|field| {
        if conjugate {
            field[index].conj()
        } else {
            field[index]
        }
    })
}

fn squared_norms(mode: [isize; 3], u: [Complex64; 3]) -> [f64; 4] {
    let k = mode.map(|m| 2.0 * std::f64::consts::PI * m as f64);
    let velocity: f64 = u.iter().map(Complex64::norm_sqr).sum();
    let curl = [
        k[1] * u[2] - k[2] * u[1],
        k[2] * u[0] - k[0] * u[2],
        k[0] * u[1] - k[1] * u[0],
    ];
    [
        velocity,
        velocity * (1.0 + k.iter().map(|q| q * q).sum::<f64>()),
        curl.iter().map(Complex64::norm_sqr).sum(),
        (k[0] * u[0] + k[1] * u[1] + k[2] * u[2]).norm_sqr(),
    ]
}

fn finish(sums: [f64; 4]) -> Norms {
    let [l2, h1, vorticity_l2, divergence_l2] = sums.map(f64::sqrt);
    Norms {
        l2,
        h1,
        vorticity_l2,
        divergence_l2,
    }
}

/// Sum the complete strict complex band and its orthogonal coarse/new subsets.
pub fn modal_oracle(
    coarse: Domain,
    fine: Domain,
    a: [&[Complex64]; 3],
    b: [&[Complex64]; 3],
) -> BandComparison {
    assert_eq!(coarse.lengths(), [1.0; 3]);
    assert_eq!(fine.lengths(), [1.0; 3]);
    let nc = coarse.layout().dimensions();
    let nf = fine.layout().dimensions();
    let half = nf.map(|n| n as isize / 2);
    let mut sums = [[0.0; 4]; 2];
    for x in (-half[0] + 1)..half[0] {
        for y in (-half[1] + 1)..half[1] {
            for z in (-half[2] + 1)..half[2] {
                let mode = [x, y, z];
                let left = value(nc, a, mode);
                let right = value(nf, b, mode);
                let u = std::array::from_fn(|axis| right[axis] - left[axis]);
                let new_mode = (0..3).any(|axis| mode[axis].unsigned_abs() >= nc[axis] / 2);
                for (sum, contribution) in sums[usize::from(new_mode)]
                    .iter_mut()
                    .zip(squared_norms(mode, u))
                {
                    *sum += contribution;
                }
            }
        }
    }
    BandComparison {
        full: finish(std::array::from_fn(|axis| sums[0][axis] + sums[1][axis])),
        common: finish(sums[0]),
        newly_resolved: finish(sums[1]),
        mean_error: std::array::from_fn(|axis| b[axis][0].re - a[axis][0].re),
    }
}

/// Compare every measured norm and preserved mean with the independent sums.
pub fn assert_oracle(actual: BandComparison, expected: BandComparison) {
    for (a, b) in [
        (actual.full, expected.full),
        (actual.common, expected.common),
        (actual.newly_resolved, expected.newly_resolved),
    ] {
        for (x, y) in [
            (a.l2, b.l2),
            (a.h1, b.h1),
            (a.vorticity_l2, b.vorticity_l2),
            (a.divergence_l2, b.divergence_l2),
        ] {
            assert!((x - y).abs() < 5e-14 * (1.0 + y), "{x} != {y}");
        }
    }
    assert_eq!(actual.mean_error, expected.mean_error);
}
