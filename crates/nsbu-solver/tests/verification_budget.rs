//! Every channel has an allocation; lost additions cannot make an over-budget ledger pass.
use nsbu_solver::verification::{
    budget::{Budget, Channel, CHANNELS},
    refinement::{Requirement, Rule},
    VerificationError,
};

fn rules(amounts: [f64; 11]) -> [Rule; 11] {
    amounts.map(|value| Rule::new(value, 0.5, 0.125, Requirement::Refinement).unwrap())
}

#[test]
fn all_channels_have_frozen_allocations_and_mandatory_refinement_cannot_be_weakened() {
    let amounts = std::array::from_fn(|index| (index + 1) as f64);
    let budget = Budget::new(66.0, rules(amounts)).unwrap();
    assert_eq!(budget.total(), 66.0);
    assert_eq!(budget.allocated(), 66.0);
    for ((channel, expected), amount) in CHANNELS
        .into_iter()
        .zip([
            true, true, false, false, false, false, false, false, true, true, true,
        ])
        .zip(amounts)
    {
        assert_eq!(channel.requires_refinement(), expected);
        assert_eq!(budget.rule(channel).budget(), amount);
        assert_eq!(budget.rule(channel).requirement(), Requirement::Refinement);
    }
    let sensitivity = Rule::new(1.0, 0.5, 0.125, Requirement::Sensitivity).unwrap();
    for channel in [
        Channel::Space,
        Channel::Time,
        Channel::Sampling,
        Channel::Reconstruction,
        Channel::Quadrature,
    ] {
        let mut changed = rules([1.0; 11]);
        changed[channel as usize] = sensitivity;
        assert_eq!(
            Budget::new(11.0, changed).unwrap_err(),
            VerificationError::InvalidRequirement
        );
    }
    for channel in [
        Channel::Method,
        Channel::ForceResolution,
        Channel::ForcePrecision,
        Channel::Arithmetic,
        Channel::ReferencePrecision,
        Channel::Transfer,
    ] {
        let mut changed = rules([1.0; 11]);
        changed[channel as usize] = sensitivity;
        assert_eq!(
            Budget::new(11.0, changed)
                .unwrap()
                .rule(channel)
                .requirement(),
            Requirement::Sensitivity
        );
    }
}

#[test]
fn exact_sums_and_both_rounding_directions_are_accounted_for_conservatively() {
    assert_eq!(
        Budget::new(11.0, rules([1.0; 11])).unwrap().allocated(),
        11.0
    );
    assert_eq!(
        Budget::new(11.0_f64.next_down(), rules([1.0; 11])).unwrap_err(),
        VerificationError::ExcessAllocation
    );
    for small in [f64::EPSILON / 8.0, 0.75 * f64::EPSILON] {
        let mut amounts = [small; 11];
        amounts[0] = 1.0;
        assert_eq!(
            Budget::new(1.0, rules(amounts)).unwrap_err(),
            VerificationError::ExcessAllocation
        );
        let upper = 1.0 + 10.0 * f64::EPSILON;
        assert_eq!(
            Budget::new(upper, rules(amounts)).unwrap().allocated(),
            upper
        );
    }
    let small = f64::from_bits(8);
    assert_eq!(
        Budget::new(f64::from_bits(88), rules([small; 11]))
            .unwrap()
            .allocated(),
        f64::from_bits(88)
    );
}

#[test]
fn invalid_totals_and_overflow_are_explicit_refusals() {
    for total in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            Budget::new(total, rules([1.0; 11])).unwrap_err(),
            VerificationError::InvalidValue
        );
    }
    assert_eq!(
        Budget::new(f64::MAX, rules([f64::MAX; 11])).unwrap_err(),
        VerificationError::ExcessAllocation
    );
    for index in [0, 10] {
        let mut amounts = [1.0; 11];
        amounts[index] = f64::MAX;
        assert_eq!(
            Budget::new(f64::MAX, rules(amounts)).unwrap_err(),
            VerificationError::ExcessAllocation
        );
    }
}
