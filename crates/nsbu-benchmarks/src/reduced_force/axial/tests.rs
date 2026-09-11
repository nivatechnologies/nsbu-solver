use super::*;
use crate::reduced_force;
use nsbu_solver::domain::TickClock;

fn time(exponent: i32, target: u128, elapsed: u128) -> BenchmarkTime {
    BenchmarkTime::new(TickClock::restore(exponent, target, elapsed, target - elapsed).unwrap())
        .unwrap()
}

fn words(sample: &ForceSample) -> Vec<u64> {
    let mut result = Vec::new();
    result.extend(sample.velocity.map(f64::to_bits));
    result.push(sample.pressure_raw.to_bits());
    result.extend(sample.force.map(f64::to_bits));
    for row in sample.momentum_terms {
        result.extend(row.map(f64::to_bits));
    }
    result.extend(sample.root_residual.map(f64::to_bits));
    if let Some(root) = sample.root {
        result.extend(
            [
                root.value,
                root.bracket[0],
                root.bracket[1],
                root.residual,
                root.error_estimate,
            ]
            .map(f64::to_bits),
        );
        result.push(root.iterations as u64);
    } else {
        result.push(u64::MAX);
    }
    result
}

#[test]
fn cached_samples_match_uncached_words_for_planes_and_nonmonotone_times() {
    let points = [
        [0.0, 0.0, 0.125],
        [0.125, -0.25, 0.125],
        [0.3, 0.1, 0.125],
        [0.0, 0.0, -0.125],
        [0.25, 0.0, -0.125],
    ];
    for elapsed in [1, 4, 2, 1] {
        let clock = time(-10, 8, elapsed);
        for z in [0.125, -0.125] {
            let root = AxialRoot::new(z, clock).unwrap().unwrap();
            for mut point in points {
                point[2] = z;
                let expected = reduced_force::evaluate(point, clock).unwrap();
                let actual = evaluate(point, clock, Some(&root)).unwrap();
                assert_eq!(
                    words(&actual),
                    words(&expected),
                    "point={point:?}, elapsed={elapsed}"
                );
                let repeated = evaluate(point, clock, Some(&root)).unwrap();
                assert_eq!(words(&repeated), words(&actual));
            }
        }
    }
    let tiny = time(-67, 1u128 << 60, (1u128 << 60) - 1);
    let point = [0.125, 0.0, 0.125];
    let root = AxialRoot::new(point[2], tiny).unwrap().unwrap();
    assert_eq!(
        words(&evaluate(point, tiny, Some(&root)).unwrap()),
        words(&reduced_force::evaluate(point, tiny).unwrap())
    );
}

#[test]
fn flat_rest_and_exterior_planes_return_zero_without_a_cache() {
    let rest = time(-10, 8, 0);
    let exterior = time(-10, 8, 1);
    for (point, clock) in [([0.0, 0.0, 0.125], rest), ([0.0, 0.0, 0.5], exterior)] {
        assert!(AxialRoot::new(point[2], clock).unwrap().is_none());
        let sample = evaluate(point, clock, None).unwrap();
        assert_eq!(sample.force, [0.0; 3]);
        assert_eq!(sample.momentum_terms, [[0.0; 4]; 3]);
        assert_eq!(sample.velocity, [0.0; 3]);
        assert_eq!(sample.pressure_raw, 0.0);
        assert!(sample.root.is_none());
    }
}

#[test]
fn active_missing_wrong_periodic_plane_stale_time_and_invalid_inputs_are_refused() {
    let clock = time(-10, 8, 1);
    let root = AxialRoot::new(0.125, clock).unwrap().unwrap();
    assert!(evaluate([0.0, 0.0, 0.125], clock, None).is_err());
    assert!(evaluate([0.0, 0.0, -0.125], clock, Some(&root)).is_err());
    assert!(evaluate([f64::NAN, 0.0, 0.125], clock, Some(&root)).is_err());
    assert!(AxialRoot::new(f64::NAN, clock).is_err());
    let later = time(-10, 8, 2);
    assert!(evaluate([0.0, 0.0, 0.125], later, Some(&root)).is_err());

    let target = 1u128 << 60;
    let a = time(-67, target, target - 1);
    let b = time(-67, target, target - 2);
    assert_eq!(a.elapsed().to_bits(), b.elapsed().to_bits());
    assert_ne!(a.remaining().to_bits(), b.remaining().to_bits());
    let root = AxialRoot::new(0.0, a).unwrap().unwrap();
    assert!(evaluate([0.0, 0.0, 0.0], b, Some(&root)).is_err());
}

#[test]
fn periodic_plane_cache_uses_canonical_z_bits() {
    let clock = time(-10, 8, 1);
    let root = AxialRoot::new(0.875, clock).unwrap().unwrap();
    let point = [0.125, -0.25, 0.875];
    let cached = evaluate(point, clock, Some(&root)).unwrap();
    let direct = reduced_force::evaluate(point, clock).unwrap();
    assert_eq!(words(&cached), words(&direct));
}
