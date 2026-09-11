//! Reduced-provider trajectories compared with the established Cartesian provider.
mod fixture_support;
mod reduced_trajectory_support;

use nsbu_solver::{
    integrators::{indicator::Tolerances, method::Method, transaction::commit_candidate},
    Complex64,
};
use reduced_trajectory_support as support;

fn tolerances() -> Tolerances {
    Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [1e-5; 2],
    }
}

fn run(method: Method, fixture: &str) {
    let (mut original, mut reduced) = support::make_pair(method);
    let mut maximum_pair = 0.0_f64;
    let mut cumulative_original = 0_usize;
    let mut cumulative_reduced = 0_usize;
    for step in 0..support::STEPS {
        let before_original = support::bits(&original.state);
        let before_reduced = support::bits(&reduced.state);
        let a = original
            .workspace
            .try_advance(
                &original.state,
                &mut original.candidate,
                support::STEP,
                tolerances(),
                &mut original.rhs,
            )
            .unwrap();
        let b = reduced
            .workspace
            .try_advance(
                &reduced.state,
                &mut reduced.candidate,
                support::STEP,
                tolerances(),
                &mut reduced.rhs,
            )
            .unwrap();
        assert!(
            a.accepted.is_some(),
            "original method={method:?} step={step}: {a:?}"
        );
        assert!(
            b.accepted.is_some(),
            "reduced method={method:?} step={step}: {b:?}"
        );
        assert_eq!(a.ticks, b.ticks);
        assert_eq!(a.rhs_calls, method.rhs_calls());
        assert_eq!(b.rhs_calls, method.rhs_calls());
        assert_eq!(original.rhs.consumption(), reduced.rhs.consumption());
        let ta = a.accepted.unwrap();
        let tb = b.accepted.unwrap();
        assert_eq!(support::bits(&original.state), before_original);
        assert_eq!(support::bits(&reduced.state), before_reduced);
        assert_ne!(
            support::bits(original.candidate.proposal(&original.state, &ta).unwrap()),
            before_original
        );
        assert_ne!(
            support::bits(reduced.candidate.proposal(&reduced.state, &tb).unwrap()),
            before_reduced
        );
        assert_eq!(original.rhs.consumption()[0], method.rhs_calls());
        assert_eq!(reduced.rhs.consumption()[0], method.rhs_calls());
        assert_eq!(original.rhs.consumption()[2], method.rhs_calls() * 13);
        assert_eq!(reduced.rhs.consumption()[2], method.rhs_calls() * 13);
        cumulative_original = cumulative_original
            .checked_add(original.rhs.consumption()[1])
            .unwrap();
        cumulative_reduced = cumulative_reduced
            .checked_add(reduced.rhs.consumption()[1])
            .unwrap();
        assert!(cumulative_original <= original.maximum_work);
        assert!(cumulative_reduced <= reduced.maximum_work);
        commit_candidate(
            original.state.plan(),
            &mut original.state,
            &mut original.candidate,
            ta,
        )
        .unwrap();
        commit_candidate(
            reduced.state.plan(),
            &mut reduced.state,
            &mut reduced.candidate,
            tb,
        )
        .unwrap();
        assert_eq!(original.state.clock(), reduced.state.clock());
        assert_eq!(original.state.accepted_steps(), (step + 1) as u128);
        assert_eq!(reduced.state.accepted_steps(), (step + 1) as u128);
        let mut maximum = 0.0_f64;
        for (x, y) in (0..3).flat_map(|axis| {
            original
                .state
                .component(axis)
                .unwrap()
                .iter()
                .zip(reduced.state.component(axis).unwrap())
        }) {
            maximum = maximum.max((x.re - y.re).abs() / (1.0 + x.re.abs()));
            maximum = maximum.max((x.im - y.im).abs() / (1.0 + x.im.abs()));
        }
        maximum_pair = maximum_pair.max(maximum);
        assert!(
            maximum < 5e-12,
            "method={method:?} step={step} reduced/original scaled error={maximum:e}"
        );
        assert_ne!(support::bits(&original.state), before_original);
        assert_ne!(support::bits(&reduced.state), before_reduced);
        let elapsed = (step as u128 + 1) * support::STEP;
        assert_eq!(original.state.clock().elapsed(), elapsed);
        assert_eq!(reduced.state.clock().elapsed(), elapsed);
    }
    assert_eq!(original.state.clock().elapsed(), support::ENDPOINT);
    assert_eq!(reduced.state.clock().elapsed(), support::ENDPOINT);
    let original_output =
        std::array::from_fn(|axis| original.state.component(axis).unwrap().to_vec());
    let reduced_output =
        std::array::from_fn(|axis| reduced.state.component(axis).unwrap().to_vec());
    fixture_support::compare(support::domain().layout(), &original_output, fixture, 5e-13);
    fixture_support::compare(support::domain().layout(), &reduced_output, fixture, 5e-13);
    let original_fixture_max = fixture_max(&original_output, fixture);
    let reduced_fixture_max = fixture_max(&reduced_output, fixture);
    println!("reduced trajectory method={method:?} steps=32 endpoint=4096 maximum_pair={maximum_pair:e} endpoint_fixture_max_original={original_fixture_max:e} endpoint_fixture_max_reduced={reduced_fixture_max:e} provider_consumption={:?} cumulative_work=[{cumulative_original},{cumulative_reduced}]", reduced.rhs.consumption());
}

fn fixture_max(output: &[Vec<Complex64>; 3], fixture: &str) -> f64 {
    let layout = support::domain().layout();
    fixture.lines().fold(0.0, |maximum, line| {
        let columns: Vec<_> = line.split('\t').collect();
        let mode = std::array::from_fn(|i| columns[i].parse::<isize>().unwrap());
        let (index, conjugate) = layout.locate(mode).unwrap();
        assert!(!conjugate);
        (0..3).fold(maximum, |maximum, component| {
            let expected = Complex64::new(
                columns[3 + 2 * component].parse().unwrap(),
                columns[4 + 2 * component].parse().unwrap(),
            );
            maximum
                .max((output[component][index] - expected).l1_norm() / (1.0 + expected.l1_norm()))
        })
    })
}

#[test]
fn cm_and_ho_reduced_trajectories_match_fixtures_and_original_each_commit() {
    run(
        Method::CoxMatthews,
        include_str!("fixtures/concentrating-n4.tsv"),
    );
    run(
        Method::HochbruckOstermann,
        include_str!("fixtures/concentrating-ho-n4.tsv"),
    );
}

#[test]
fn rejected_reduced_attempt_charges_work_without_changing_committed_state() {
    let (_, mut trajectory) = support::make_pair(Method::CoxMatthews);
    let before = support::bits(&trajectory.state);
    let clock = trajectory.state.clock();
    let attempts = trajectory.state.accepted_steps();
    let strict = Tolerances {
        absolute: [1e-30, 1e-30],
        relative: [1e-30; 2],
    };
    let result = trajectory
        .workspace
        .try_advance(
            &trajectory.state,
            &mut trajectory.candidate,
            support::STEP,
            strict,
            &mut trajectory.rhs,
        )
        .unwrap();
    assert!(result.accepted.is_none());
    assert_eq!(trajectory.state.clock(), clock);
    assert_eq!(trajectory.state.accepted_steps(), attempts);
    assert_eq!(support::bits(&trajectory.state), before);
    assert!(trajectory.rhs.consumption()[0] > 0);
    assert!(trajectory.rhs.consumption()[1] > 0);
    assert_eq!(
        trajectory.rhs.consumption()[2],
        trajectory.rhs.consumption()[0] * 13
    );
}
