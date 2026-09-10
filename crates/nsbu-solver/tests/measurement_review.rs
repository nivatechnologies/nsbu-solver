//! Streaming review preserves missing channels, exact ordering and every failed observation.
use nsbu_solver::{
    domain::TickClock,
    verification::{
        budget::Budget,
        observation::{ComparisonScope, Observation},
        policy::{ObservablePolicy, Policies},
        reconstruction::{OffStageProbe, ProbeRefinement, ReconstructionSamples},
        refinement::{Evidence, Finding, Requirement, Rule},
        review::{required_records, MeasurementReview, ReviewStatus},
        times::TestedTimes,
        VerificationError,
    },
};

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-10, 128, elapsed, 128 - elapsed).unwrap()
}

struct Fixture {
    policies: [ObservablePolicy; 2],
    coarse: [TickClock; 2],
    middle: [TickClock; 3],
    fine: [TickClock; 4],
    probes: [ProbeRefinement; 1],
    channels: [Evidence; 11],
}
impl Fixture {
    fn new() -> Self {
        let policies = [1.0, 2.0].map(|amount| {
            let r = Rule::new(amount, 0.5, 0.125, Requirement::Refinement).unwrap();
            let s = Rule::new(amount, 0.5, 0.125, Requirement::Sensitivity).unwrap();
            ObservablePolicy {
                key: (amount * 10.0) as u32,
                budget: Budget::new(amount * 22.0, [r, r, s, s, s, s, s, s, r, r, r]).unwrap(),
            }
        });
        let levels = [32, 16, 8]
            .map(|step| OffStageProbe::new([0, step, 2 * step].map(clock), clock(3)).unwrap());
        let r = Evidence::Sequence([0.25, 0.0625]);
        let s = Evidence::Pair(0.0625);
        Self {
            policies,
            coarse: [0, 64].map(clock),
            middle: [0, 16, 64].map(clock),
            fine: [0, 3, 16, 64].map(clock),
            probes: [ProbeRefinement::new(levels).unwrap()],
            channels: [r, r, s, s, s, s, s, s, r, r, r],
        }
    }

    fn review(&self, attempts: usize) -> Result<MeasurementReview<'_>, VerificationError> {
        let times = [&self.coarse[..], &self.middle[..], &self.fine[..]]
            .map(|times| TestedTimes::new(times, 4).unwrap());
        MeasurementReview::new(
            Policies::new(&self.policies, 1).unwrap(),
            times,
            ReconstructionSamples::new(&self.probes, times[2], 1).unwrap(),
            attempts,
        )
    }

    fn observation(&self, index: usize) -> Observation<'_> {
        Observation {
            time: self.fine[index / 2],
            key: self.policies[index % 2].key,
            comparison: ComparisonScope::FullField,
            tracking_error: Some(1.0),
            channels: &self.channels,
        }
    }
}

#[test]
fn complete_numerical_findings_only_reach_the_separate_lineage_review() {
    let fixture = Fixture::new();
    let mut review = fixture.review(9).unwrap();
    assert_eq!(review.status(), ReviewStatus::Incomplete);
    assert_eq!(review.progress().required, 8);
    assert_eq!(
        review.reconstruction().as_slice()[0].levels()[2].time(),
        clock(3)
    );
    for index in 0..8 {
        let observation = fixture.observation(index);
        assert_eq!(review.next(), Some((observation.time, observation.key)));
        let findings = review.push(observation).unwrap();
        assert_eq!(findings.tracking, Finding::TrackingBelowBudget);
        assert!(findings.passes());
    }
    assert_eq!(review.next(), None);
    assert_eq!(review.status(), ReviewStatus::ReadyForLineageReview);
    assert_eq!(review.progress().reviewed, 8);
    assert_eq!(review.progress().failed, 0);
    assert_eq!(review.progress().attempts_left, 1);
    assert_eq!(
        review.push(fixture.observation(0)).unwrap_err(),
        VerificationError::UnexpectedObservation
    );
    assert_eq!(review.status(), ReviewStatus::ReadyForLineageReview);
}

#[test]
fn every_missing_channel_remains_a_permanent_failure() {
    let fixture = Fixture::new();
    for missing in 0..11 {
        let mut review = fixture.review(8).unwrap();
        let mut channels = fixture.channels;
        channels[missing] = Evidence::Missing;
        let findings = review
            .push(Observation {
                channels: &channels,
                ..fixture.observation(0)
            })
            .unwrap();
        assert_eq!(findings.channels[missing], Finding::MissingEvidence);
        assert!(!findings.passes());
        for index in 1..8 {
            assert!(review.push(fixture.observation(index)).unwrap().passes());
        }
        assert_eq!(review.status(), ReviewStatus::Rejected);
        assert_eq!(review.progress().failed, 1);
    }
}

#[test]
fn missing_tracking_and_failed_global_errors_cannot_be_hidden_by_later_success() {
    let fixture = Fixture::new();
    for (tracking_error, expected) in [
        (None, Finding::MissingEvidence),
        (Some(22.0), Finding::AboveBudget),
    ] {
        let mut review = fixture.review(9).unwrap();
        let input = Observation {
            tracking_error,
            ..fixture.observation(0)
        };
        let finding = review.push(input).unwrap();
        assert_eq!(finding.tracking, expected);
        assert!(!finding.passes());
        assert_eq!(
            review.push(input).unwrap_err(),
            VerificationError::UnexpectedObservation
        );
        assert_eq!(review.progress().reviewed, 1);
        assert_eq!(review.progress().failed, 1);
        for index in 1..8 {
            let input = Observation {
                tracking_error: Some(21.0),
                ..fixture.observation(index)
            };
            assert!(review.push(input).unwrap().passes());
        }
        assert_eq!(review.status(), ReviewStatus::Rejected);
        assert_eq!(review.progress().failed, 1);
    }
}

#[test]
fn malformed_records_spend_work_without_committing_partial_findings() {
    let fixture = Fixture::new();
    let mut review = fixture.review(12).unwrap();
    let mut invalid_channels = fixture.channels;
    invalid_channels[10] = Evidence::Pair(f64::NAN);
    for (input, expected) in [
        (
            Observation {
                key: 999,
                ..fixture.observation(0)
            },
            VerificationError::UnexpectedObservation,
        ),
        (
            Observation {
                time: clock(3),
                ..fixture.observation(0)
            },
            VerificationError::UnexpectedObservation,
        ),
        (
            Observation {
                tracking_error: Some(f64::NAN),
                ..fixture.observation(0)
            },
            VerificationError::InvalidValue,
        ),
        (
            Observation {
                tracking_error: Some(22.0),
                channels: &invalid_channels,
                ..fixture.observation(0)
            },
            VerificationError::InvalidValue,
        ),
    ] {
        assert_eq!(review.push(input).unwrap_err(), expected);
        assert_eq!(review.progress().reviewed, 0);
        assert_eq!(review.progress().failed, 0);
        assert_eq!(review.next(), Some((clock(0), 10)));
    }
    assert_eq!(review.progress().attempts_left, 8);
    let mut channels = fixture.channels;
    channels[0] = Evidence::Floor {
        changes: [0.0; 2],
        bound: 0.0,
        analysis: [1; 32],
    };
    let findings = review
        .push(Observation {
            channels: &channels,
            ..fixture.observation(0)
        })
        .unwrap();
    assert_eq!(findings.channels[0], Finding::SubordinateFloor);
    assert!(findings.passes());
    for index in 1..8 {
        assert!(review.push(fixture.observation(index)).unwrap().passes());
    }
    assert_eq!(review.status(), ReviewStatus::ReadyForLineageReview);
}

#[test]
fn attempt_exhaustion_preserves_missing_records_instead_of_accepting_a_partial_table() {
    let fixture = Fixture::new();
    let mut review = fixture.review(8).unwrap();
    assert_eq!(
        review.push(fixture.observation(1)).unwrap_err(),
        VerificationError::UnexpectedObservation
    );
    for index in 0..7 {
        assert!(review.push(fixture.observation(index)).unwrap().passes());
    }
    assert_eq!(review.status(), ReviewStatus::BudgetExceeded);
    assert_eq!(review.progress().reviewed, 7);
    let before = review.progress();
    assert_eq!(
        review.push(fixture.observation(7)).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    assert_eq!(review.progress(), before);
    assert_eq!(review.next(), Some((clock(64), 20)));
}

#[test]
fn policy_and_record_count_preflight_reject_missing_duplicate_and_excess_work() {
    let fixture = Fixture::new();
    assert_eq!(
        Policies::new(&[], 0).unwrap_err(),
        VerificationError::MissingPolicy
    );
    let duplicate = [fixture.policies[0]; 2];
    assert_eq!(
        Policies::new(&duplicate, 1).unwrap_err(),
        VerificationError::DuplicateObservable
    );
    assert_eq!(
        Policies::new(&fixture.policies, 0).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    assert_eq!(
        fixture.review(7).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    assert_eq!(
        required_records(0, 2).unwrap_err(),
        VerificationError::InvalidValue
    );
    assert_eq!(
        required_records(1, 1).unwrap_err(),
        VerificationError::InvalidValue
    );
    assert_eq!(
        required_records(usize::MAX, 2).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    assert_eq!(required_records(2, 4).unwrap(), 8);
    let three = [
        fixture.policies[0],
        fixture.policies[1],
        ObservablePolicy {
            key: 30,
            budget: fixture.policies[0].budget,
        },
    ];
    for cap in [1, 2] {
        assert_eq!(
            Policies::new(&three, cap).unwrap_err(),
            VerificationError::CapacityExceeded
        );
    }
    assert_eq!(Policies::new(&three, 3).unwrap().as_slice().len(), 3);
}

#[test]
fn cropped_or_aligned_agreement_cannot_be_entered_as_primary_evidence() {
    let fixture = Fixture::new();
    let mut review = fixture.review(10).unwrap();
    for comparison in [ComparisonScope::CommonBand, ComparisonScope::Aligned] {
        let input = Observation {
            tracking_error: Some(0.0),
            comparison,
            ..fixture.observation(0)
        };
        assert_eq!(
            review.push(input).unwrap_err(),
            VerificationError::InvalidComparisonScope
        );
        assert_eq!(review.progress().reviewed, 0);
        assert_eq!(review.status(), ReviewStatus::Incomplete);
    }
    for index in 0..8 {
        assert!(review.push(fixture.observation(index)).unwrap().passes());
    }
    assert_eq!(review.status(), ReviewStatus::ReadyForLineageReview);
}

#[test]
fn the_complete_time_and_reconstruction_manifest_must_match_the_review() {
    let fixture = Fixture::new();
    let policies = Policies::new(&fixture.policies, 1).unwrap();
    let [a, b, c] = [&fixture.coarse[..], &fixture.middle[..], &fixture.fine[..]]
        .map(|values| TestedTimes::new(values, 4).unwrap());
    let probes = ReconstructionSamples::new(&fixture.probes, c, 1).unwrap();
    for sets in [[a, a, c], [a, b, b]] {
        assert_eq!(
            MeasurementReview::new(policies, sets, probes, 8).unwrap_err(),
            VerificationError::InvalidTimes
        );
    }
    let extra = [0, 3, 8, 16, 64].map(clock);
    let other = TestedTimes::new(&extra, 5).unwrap();
    assert_eq!(
        MeasurementReview::new(policies, [a, b, other], probes, 10).unwrap_err(),
        VerificationError::InvalidTimes
    );
}
