//! Frozen exact-v2 review-profile admission and source-geometry controls.
use nsbu_benchmarks::v2_experiment::{
    diagnostic::{DiagnosticDriver, StartupProfile},
    probes::ProbePlan,
    review_profile::{
        semantics_identity, AdmittedProfile, MandatoryObservableGap, ObservableQuantity,
        ObservableSource, ObservableStatistic, ObservableUnits, ProfileCaps, ProfileError,
        ProfileInputs, ProfileStatus, ReviewGeometry, MANDATORY_GAPS, OBSERVABLES,
        OBSERVABLE_COUNT, PROBLEM_IDENTITY, SEMANTICS_BYTES,
    },
    FamilyPlan,
};
use nsbu_solver::{
    domain::TickClock,
    verification::{
        budget::{Budget, CHANNELS},
        policy::ObservablePolicy,
        refinement::{Requirement, Rule},
        review::ReviewStatus,
        times::TestedTimes,
    },
};

const CAP: usize = 256 * 1024 * 1024;
const ROWS: usize = OBSERVABLE_COUNT * 7;
const PAIRS: usize = OBSERVABLE_COUNT * (OBSERVABLE_COUNT - 1) / 2;

fn policy() -> Budget {
    let rules = CHANNELS.map(|channel| {
        Rule::new(
            1.0,
            0.5,
            0.125,
            if channel.requires_refinement() {
                Requirement::Refinement
            } else {
                Requirement::Sensitivity
            },
        )
        .unwrap()
    });
    Budget::new(12.0, rules).unwrap()
}
fn policies() -> [ObservablePolicy; OBSERVABLE_COUNT] {
    let budget = policy();
    OBSERVABLES.map(|observable| ObservablePolicy {
        key: observable.key,
        budget,
    })
}
fn geometry() -> ReviewGeometry {
    let profile = StartupProfile::new().unwrap();
    let plan = profile.plan(CAP).unwrap();
    ReviewGeometry::startup(plan.probe_plan()).unwrap()
}
fn caps(bytes: usize) -> ProfileCaps {
    ProfileCaps {
        policy_pair_checks: PAIRS,
        protocol_bytes: bytes,
        required_rows: ROWS,
        review_attempts: ROWS + 2,
    }
}
fn input(policies: &[ObservablePolicy]) -> ProfileInputs<'_> {
    ProfileInputs {
        problem: PROBLEM_IDENTITY,
        semantics: semantics_identity(),
        policies,
    }
}

#[test]
fn actual_probe_plan_geometry_admits_the_unpopulated_generic_protocol() {
    let geometry = geometry();
    let policies = policies();
    let admitted = AdmittedProfile::new(&geometry, input(&policies), caps(usize::MAX)).unwrap();
    assert_eq!(
        admitted.status(),
        ProfileStatus::PartialUnpopulatedDiagnostic
    );
    assert_eq!(admitted.bounds().required_rows, ROWS);
    assert_eq!(admitted.bounds().policy_pair_checks, PAIRS);
    assert_eq!(admitted.bounds().semantics_bytes, SEMANTICS_BYTES);
    assert_eq!(admitted.bounds().review_attempts, ROWS + 2);
    assert_eq!(admitted.semantics_identity(), semantics_identity());
    assert_eq!(
        admitted.review().unwrap().status(),
        ReviewStatus::Incomplete
    );

    let first = admitted.required_row(0).unwrap();
    assert_eq!(first.clock.elapsed(), 0);
    assert_eq!(first.observable, OBSERVABLES[0]);
    let next_clock = admitted.required_row(OBSERVABLE_COUNT).unwrap();
    assert_eq!(next_clock.clock.elapsed(), 7);
    let last = admitted.required_row(ROWS - 1).unwrap();
    assert_eq!(last.clock.elapsed(), 128);
    assert_eq!(last.observable, OBSERVABLES[OBSERVABLE_COUNT - 1]);
    assert_eq!(admitted.required_row(ROWS), None);

    let mut canonical = vec![0xa5; admitted.bounds().protocol_bytes + 1];
    let written = admitted.write_canonical(&mut canonical).unwrap();
    assert_eq!(written, admitted.bounds().protocol_bytes);
    assert_eq!(canonical[written], 0xa5);
    assert_eq!(
        admitted.identity(),
        [
            0x6d, 0x0b, 0xf5, 0x45, 0xdc, 0x8f, 0x66, 0x04, 0x14, 0x98, 0xfc, 0xee, 0x0b, 0x2d,
            0xa1, 0x5b, 0x3a, 0x58, 0xd2, 0xfb, 0x03, 0xae, 0xc7, 0xcb, 0x6d, 0xcb, 0xf2, 0x03,
            0x52, 0x29, 0x94, 0xba,
        ]
    );
}

#[test]
fn allocated_driver_plan_retains_the_bound_geometry_identities() {
    let startup = StartupProfile::new().unwrap();
    let plan = startup.plan(CAP).unwrap();
    let driver = DiagnosticDriver::new(plan).unwrap();
    let geometry = ReviewGeometry::startup(driver.plan().probe_plan()).unwrap();
    assert_eq!(
        geometry.family_identity(),
        driver.plan().family_plan().identity()
    );
    assert_eq!(
        geometry.probe_identity(),
        driver.plan().probe_plan().identity()
    );
}

#[test]
fn all_actual_offstage_histories_and_inventory_semantics_are_exact() {
    let geometry = geometry();
    assert_eq!(
        semantics_identity(),
        [
            0xe7, 0x85, 0x76, 0x81, 0x3a, 0x6b, 0xc3, 0xc0, 0xde, 0x69, 0x96, 0x7b, 0x67, 0x9a,
            0x92, 0x47, 0x07, 0xa0, 0x26, 0xca, 0x96, 0x49, 0xcd, 0x98, 0x8f, 0x58, 0xd5, 0x08,
            0x63, 0x77, 0x69, 0x2e,
        ]
    );
    let expected = [
        (7, [[0, 64, 128], [0, 32, 64], [0, 16, 32]]),
        (63, [[0, 64, 128], [0, 32, 64], [32, 48, 64]]),
        (95, [[0, 64, 128], [32, 64, 96], [64, 80, 96]]),
        (127, [[0, 64, 128], [64, 96, 128], [96, 112, 128]]),
    ];
    for (actual, (probe, nodes)) in geometry.probes().iter().zip(expected) {
        for (level, expected_nodes) in actual.levels().into_iter().zip(nodes) {
            assert_eq!(level.time().elapsed(), probe);
            assert_eq!(level.nodes().map(|clock| clock.elapsed()), expected_nodes);
        }
    }
    assert_eq!(OBSERVABLES.len(), 88);
    assert!(OBSERVABLES
        .windows(2)
        .all(|pair| pair[1].key == pair[0].key + 1));
    assert_eq!(OBSERVABLES[0].source, ObservableSource::AcceptedSpectral);
    assert_eq!(OBSERVABLES[1].statistic, ObservableStatistic::FourierH1);
    assert_eq!(OBSERVABLES[1].units, ObservableUnits::VelocityH1);
    assert_eq!(
        OBSERVABLES[OBSERVABLE_COUNT - 6].quantity,
        ObservableQuantity::MomentumResidual
    );
    assert!(MANDATORY_GAPS.contains(&MandatoryObservableGap::PressureGauge));
    assert!(MANDATORY_GAPS.contains(&MandatoryObservableGap::CoreNominalCoverage));
    assert!(MANDATORY_GAPS.contains(&MandatoryObservableGap::CollarVolumeCoverage));
}

#[test]
fn wrong_identity_inventory_and_every_short_cap_fail_before_review() {
    let geometry = geometry();
    let policies = policies();
    let mut wrong = input(&policies);
    wrong.problem[0] ^= 1;
    assert_eq!(
        AdmittedProfile::new(&geometry, wrong, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidProblem
    );
    wrong = input(&policies);
    wrong.semantics[0] ^= 1;
    assert_eq!(
        AdmittedProfile::new(&geometry, wrong, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidSemantics
    );

    let mut reordered = policies;
    reordered.swap(0, 1);
    assert_eq!(
        AdmittedProfile::new(&geometry, input(&reordered), caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidInventory
    );
    assert_eq!(
        AdmittedProfile::new(
            &geometry,
            input(&policies[..OBSERVABLE_COUNT - 1]),
            caps(usize::MAX)
        )
        .unwrap_err(),
        ProfileError::InvalidInventory
    );
    let mut duplicate = policies;
    duplicate[1].key = duplicate[0].key;
    assert_eq!(
        AdmittedProfile::new(&geometry, input(&duplicate), caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidInventory
    );
    let admitted = AdmittedProfile::new(&geometry, input(&policies), caps(usize::MAX)).unwrap();
    let exact = admitted.bounds().protocol_bytes;
    for short in [
        ProfileCaps {
            policy_pair_checks: PAIRS - 1,
            ..caps(exact)
        },
        ProfileCaps {
            required_rows: ROWS - 1,
            ..caps(exact)
        },
        ProfileCaps {
            review_attempts: ROWS - 1,
            ..caps(exact)
        },
        caps(exact - 1),
    ] {
        assert!(AdmittedProfile::new(&geometry, input(&policies), short).is_err());
    }
    let mut output = vec![0x5a; exact - 1];
    assert!(admitted.write_canonical(&mut output).is_err());
    assert!(output.iter().all(|byte| *byte == 0x5a));
}

#[test]
fn a_foreign_probe_manifest_cannot_supply_the_frozen_geometry() {
    let startup = StartupProfile::new().unwrap();
    let original = startup.plan(CAP).unwrap();
    let family: FamilyPlan<'_> = original.family_plan();
    let clocks = [0, 7, 31, 63, 64, 95, 127, 128]
        .map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let times = TestedTimes::new(&clocks, clocks.len()).unwrap();
    let foreign = ProbePlan::new(family, times, clocks.len(), CAP).unwrap();
    assert_eq!(
        ReviewGeometry::startup(foreign).unwrap_err(),
        ProfileError::InvalidGeometry
    );
}
