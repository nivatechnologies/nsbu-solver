use super::*;

type EvolutionMutation = fn(&mut Evolution);

fn closed_r6_evolution() -> Evolution {
    Evolution {
        case_sha256: "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e".into(),
        quantum_exponent: -20,
        clock_target: 8192,
        comparison_endpoint: 4096,
        lengths: [1.0; 3],
        viscosity: 1.0,
        method: "cox-matthews".into(),
        integration_force_dimensions: [512; 3],
        schedule: vec![
            ScheduleSegment {
                from_inclusive: 0,
                until_exclusive: 2048,
                step_ticks: 64,
            },
            ScheduleSegment {
                from_inclusive: 2048,
                until_exclusive: 4096,
                step_ticks: 128,
            },
        ],
        absolute_tolerances: [1e-5, 1e-4],
        relative_tolerances: [1e-5, 1e-5],
    }
}

#[test]
fn closed_r6_evolution_is_admitted() {
    assert!(crate::m512_spatial::is_r6_evolution(&closed_r6_evolution()));
}

#[test]
fn r6_evolution_refuses_every_single_field_deviation() {
    let deviations: Vec<(&str, EvolutionMutation)> = vec![
        ("case_sha256", |value| value.case_sha256 = "d".repeat(64)),
        ("quantum_exponent", |value| value.quantum_exponent = -21),
        ("clock_target", |value| value.clock_target = 8191),
        ("comparison_endpoint", |value| {
            value.comparison_endpoint = 4095
        }),
        ("lengths", |value| value.lengths = [2.0; 3]),
        ("viscosity", |value| value.viscosity = 0.5),
        ("viscosity_non_bit_equal", |value| {
            value.viscosity = f64::NAN
        }),
        ("method", |value| {
            value.method = "hochbruck-ostermann".into()
        }),
        ("integration_force_dimensions", |value| {
            value.integration_force_dimensions = [384; 3]
        }),
        ("schedule_step_ticks", |value| {
            value.schedule[0].step_ticks = 32
        }),
        ("schedule_segment_count", |value| value.schedule.truncate(1)),
        ("schedule_second_segment", |value| {
            value.schedule[1].until_exclusive = 4097
        }),
        ("absolute_tolerances", |value| {
            value.absolute_tolerances = [1e-6, 1e-4]
        }),
        ("relative_tolerances", |value| {
            value.relative_tolerances = [1e-6, 1e-5]
        }),
    ];
    for (field, mutate) in deviations {
        let mut evolution = closed_r6_evolution();
        mutate(&mut evolution);
        assert_ne!(evolution, closed_r6_evolution(), "{field} was a no-op");
        assert!(
            !crate::m512_spatial::is_r6_evolution(&evolution),
            "{field} deviation was admitted"
        );
    }
}

#[test]
fn r6_evolution_deviation_still_refuses_the_manifest_pair_on_both_sides() {
    let root = root("m512-spatial-evolution-gate");
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 6, "right");
    enable_matched_m512_spatial(&mut left, &mut right);
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    let mutations: Vec<EvolutionMutation> = vec![
        |value| value.viscosity = 0.5,
        |value| value.schedule[1].until_exclusive = 4097,
        |value| value.absolute_tolerances = [1e-5, 1e-3],
        |value| value.integration_force_dimensions = [512, 512, 511],
    ];
    for mutate in mutations {
        let mut tampered_left = left.clone();
        let mut tampered_right = right.clone();
        mutate(&mut tampered_left.evolution);
        mutate(&mut tampered_right.evolution);
        assert_eq!(
            tampered_left.evolution, tampered_right.evolution,
            "both sides must stay equal so the refusal binds is_r6_evolution"
        );
        assert_eq!(
            compare::validate_manifest_pair(&tampered_left, &tampered_right).unwrap_err(),
            "matched M512 spatial diagnostic contract mismatch"
        );
    }
}
