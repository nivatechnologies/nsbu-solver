//! Plateaus, missing channels and failed budgets cannot masquerade as convergence.
use nsbu_solver::verification::{
    refinement::{Evidence, Finding, Requirement, Rule},
    VerificationError,
};

fn rule(requirement: Requirement) -> Rule {
    Rule::new(8.0, 0.5, 0.125, requirement).unwrap()
}

#[test]
fn independent_sensitivity_and_three_level_refinement_have_distinct_requirements() {
    let refinement = rule(Requirement::Refinement);
    let sensitivity = rule(Requirement::Sensitivity);
    assert_eq!(
        refinement.evaluate(Evidence::Missing).unwrap(),
        Finding::MissingEvidence
    );
    assert_eq!(
        sensitivity.evaluate(Evidence::Missing).unwrap(),
        Finding::MissingEvidence
    );
    for change in [0.0, 1.0, f64::from_bits(8.0_f64.to_bits() - 1)] {
        assert_eq!(
            refinement.evaluate(Evidence::Pair(change)).unwrap(),
            Finding::MissingEvidence
        );
        assert_eq!(
            sensitivity.evaluate(Evidence::Pair(change)).unwrap(),
            Finding::SensitivityBelowBudget
        );
    }
    for change in [8.0, 9.0] {
        assert_eq!(
            sensitivity.evaluate(Evidence::Pair(change)).unwrap(),
            Finding::AboveBudget
        );
    }
    for policy in [refinement, sensitivity] {
        assert_eq!(
            policy.evaluate(Evidence::Sequence([16.0, 7.0])).unwrap(),
            Finding::RefinementBelowBudget
        );
        assert_eq!(
            policy.evaluate(Evidence::Sequence([32.0, 8.0])).unwrap(),
            Finding::AboveBudget
        );
        for changes in [[0.0, 0.0], [1.0, 1.0], [2.0, 1.0], [1.0, 2.0]] {
            assert_eq!(
                policy.evaluate(Evidence::Sequence(changes)).unwrap(),
                Finding::ConvergenceInconclusive
            );
        }
        assert_eq!(
            policy.evaluate(Evidence::Sequence([2.0, 0.0])).unwrap(),
            Finding::RefinementBelowBudget
        );
    }
}

#[test]
fn a_subordinate_floor_requires_separate_analysis_and_controls_both_changes() {
    let policy = rule(Requirement::Refinement);
    let mut analysis = [0; 32];
    analysis[31] = 1;
    for (changes, bound, expected) in [
        ([0.0, 0.0], 0.0, Finding::SubordinateFloor),
        ([0.5, 0.25], 0.5, Finding::SubordinateFloor),
        ([0.25, 0.5], 0.5, Finding::SubordinateFloor),
        ([0.5, 0.5], 1.0, Finding::ConvergenceInconclusive),
        ([0.75, 0.25], 0.5, Finding::ConvergenceInconclusive),
        ([0.25, 0.75], 0.5, Finding::ConvergenceInconclusive),
        ([8.0, 0.0], 0.0, Finding::AboveBudget),
        ([0.0, 8.0], 8.0, Finding::AboveBudget),
    ] {
        assert_eq!(
            policy
                .evaluate(Evidence::Floor {
                    changes,
                    bound,
                    analysis
                })
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        policy
            .evaluate(Evidence::Floor {
                changes: [0.0; 2],
                bound: 0.0,
                analysis: [0; 32]
            })
            .unwrap_err(),
        VerificationError::MissingAnalysis
    );
}

#[test]
fn nonfinite_and_negative_measurements_are_refused_in_every_position() {
    let policy = rule(Requirement::Refinement);
    for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for evidence in [
            Evidence::Pair(invalid),
            Evidence::Sequence([invalid, 1.0]),
            Evidence::Sequence([1.0, invalid]),
            Evidence::Floor {
                changes: [invalid, 0.0],
                bound: 1.0,
                analysis: [1; 32],
            },
            Evidence::Floor {
                changes: [0.0, invalid],
                bound: 1.0,
                analysis: [1; 32],
            },
            Evidence::Floor {
                changes: [0.0; 2],
                bound: invalid,
                analysis: [1; 32],
            },
        ] {
            assert_eq!(
                policy.evaluate(evidence).unwrap_err(),
                VerificationError::InvalidValue
            );
        }
    }
}

#[test]
fn policy_admission_and_underflow_preserve_strict_subordination() {
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            Rule::new(invalid, 0.5, 0.125, Requirement::Refinement).unwrap_err(),
            VerificationError::InvalidValue
        );
    }
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, 1.0, 2.0] {
        assert_eq!(
            Rule::new(8.0, invalid, 0.125, Requirement::Refinement).unwrap_err(),
            VerificationError::InvalidValue
        );
        assert_eq!(
            Rule::new(8.0, 0.5, invalid, Requirement::Refinement).unwrap_err(),
            VerificationError::InvalidValue
        );
    }
    let tiny = f64::from_bits(1);
    assert_eq!(
        Rule::new(tiny, 0.5, 0.5, Requirement::Refinement).unwrap_err(),
        VerificationError::InvalidValue
    );
    let small = Rule::new(2.0 * tiny, 0.5, 0.5, Requirement::Refinement).unwrap();
    assert_eq!(
        small.evaluate(Evidence::Sequence([tiny, 0.0])).unwrap(),
        Finding::ConvergenceInconclusive
    );
}

#[test]
fn findings_preserve_the_difference_between_passed_and_unresolved_checks() {
    for finding in [
        Finding::MissingEvidence,
        Finding::AboveBudget,
        Finding::ConvergenceInconclusive,
    ] {
        assert!(!finding.passes());
    }
    for finding in [
        Finding::SensitivityBelowBudget,
        Finding::RefinementBelowBudget,
        Finding::SubordinateFloor,
        Finding::TrackingBelowBudget,
    ] {
        assert!(finding.passes());
    }
}
