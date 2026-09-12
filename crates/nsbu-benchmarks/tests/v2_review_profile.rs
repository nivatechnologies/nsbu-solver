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
use sha2::{Digest, Sha256};

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
    geometry_from(plan.probe_plan())
}
fn geometry_from(plan: ProbePlan<'_>) -> ReviewGeometry {
    let startup = ReviewGeometry::startup(plan).unwrap();
    let coarse = [0, 64, 128].map(tick);
    let middle = [0, 63, 64, 127, 128].map(tick);
    let fine = [0, 7, 63, 64, 95, 127, 128].map(tick);
    ReviewGeometry::from_manifests(plan, coarse, middle, fine, *startup.probes()).unwrap()
}
fn tick(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn caps(bytes: usize) -> ProfileCaps {
    ProfileCaps {
        policy_pair_checks: PAIRS,
        protocol_bytes: bytes,
        required_rows: ROWS,
        review_attempts: ROWS + 2,
    }
}
fn input<'a>(geometry: &ReviewGeometry, policies: &'a [ObservablePolicy]) -> ProfileInputs<'a> {
    ProfileInputs {
        problem: PROBLEM_IDENTITY,
        semantics: semantics_identity(),
        family: geometry.family_identity(),
        probe: geometry.probe_identity(),
        policies,
    }
}

#[test]
fn actual_probe_plan_geometry_admits_the_unpopulated_generic_protocol() {
    let geometry = geometry();
    let policies = policies();
    let admitted =
        AdmittedProfile::new(&geometry, input(&geometry, &policies), caps(usize::MAX)).unwrap();
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
    let prefix = b"NSBUV2REVIEWPROFILE0001".len() + 64;
    let generic: [u8; 32] = Sha256::digest(&canonical[prefix..written]).into();
    let mut independent = Sha256::new();
    independent.update(b"NSBUV2REVIEWPROFILE0001");
    independent.update(admitted.family_identity());
    independent.update(admitted.probe_identity());
    independent.update(generic);
    assert_eq!(
        admitted.identity(),
        <[u8; 32]>::from(independent.finalize())
    );
    assert_eq!(
        admitted.identity(),
        [
            0x67, 0xfa, 0xab, 0x47, 0x38, 0x0d, 0x63, 0x74, 0xf3, 0x84, 0xde, 0x37, 0x72, 0x4c,
            0x4a, 0xc4, 0x81, 0x3a, 0x76, 0x78, 0x25, 0x59, 0x5c, 0x8e, 0x60, 0xc7, 0xb0, 0xf8,
            0xd6, 0xe1, 0x19, 0x7a,
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
fn caller_manifests_must_be_strictly_nested_complete_and_node_bound() {
    let startup = StartupProfile::new().unwrap();
    let plan = startup.plan(CAP).unwrap().probe_plan();
    let geometry = ReviewGeometry::startup(plan).unwrap();
    let coarse = [0, 64, 128].map(tick);
    let middle = [0, 63, 64, 127, 128].map(tick);
    let fine = [0, 7, 63, 64, 95, 127, 128].map(tick);
    assert_eq!(
        ReviewGeometry::from_manifests(plan, coarse, middle, fine, *geometry.probes())
            .unwrap()
            .probe_identity(),
        plan.identity()
    );

    let nonnested = [0, 32, 64, 96, 128].map(tick);
    assert_eq!(
        ReviewGeometry::from_manifests(plan, coarse, nonnested, fine, *geometry.probes())
            .unwrap_err(),
        ProfileError::InvalidGeometry
    );
    let duplicate = [0, 7, 63, 64, 95, 127, 127].map(tick);
    assert_eq!(
        ReviewGeometry::from_manifests(plan, coarse, middle, duplicate, *geometry.probes())
            .unwrap_err(),
        ProfileError::Verification(nsbu_solver::verification::VerificationError::InvalidTimes)
    );
    let missing_accepted = [0, 7, 63, 65, 95, 127, 128].map(tick);
    assert_eq!(
        ReviewGeometry::from_manifests(plan, coarse, middle, missing_accepted, *geometry.probes())
            .unwrap_err(),
        ProfileError::InvalidGeometry
    );
    let mut wrong_nodes = *geometry.probes();
    wrong_nodes.swap(0, 1);
    assert_eq!(
        ReviewGeometry::from_manifests(plan, coarse, middle, fine, wrong_nodes).unwrap_err(),
        ProfileError::InvalidGeometry
    );
}

#[test]
fn caller_cannot_relabel_geometry_as_a_foreign_family() {
    let startup = StartupProfile::new().unwrap();
    let original = startup.plan(CAP).unwrap();
    let geometry = geometry_from(original.probe_plan());
    let mut changed = original.family_plan().settings();
    changed.advective_limit = 0.31;
    let foreign_family = FamilyPlan::new(
        changed,
        TestedTimes::new(startup.accepted_times(), 3).unwrap(),
        CAP,
    )
    .unwrap();
    let foreign = ProbePlan::new(
        foreign_family,
        TestedTimes::new(startup.manifest(), 7).unwrap(),
        7,
        CAP,
    )
    .unwrap();
    let policies = policies();
    let foreign_inputs = ProfileInputs {
        family: foreign.family_plan().identity(),
        probe: foreign.identity(),
        ..input(&geometry, &policies)
    };
    assert_eq!(
        AdmittedProfile::new(&geometry, foreign_inputs, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidGeometry
    );
}

#[test]
fn all_admitted_offstage_geometries_and_inventory_semantics_are_exact() {
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
fn complete_identity_distinguishes_equal_manifests_from_an_altered_family() {
    let startup = StartupProfile::new().unwrap();
    let original = startup.plan(CAP).unwrap();
    let mut changed = original.family_plan().settings();
    changed.advective_limit = 0.31;
    let family = FamilyPlan::new(
        changed,
        TestedTimes::new(startup.accepted_times(), 3).unwrap(),
        CAP,
    )
    .unwrap();
    let probe = ProbePlan::new(
        family,
        TestedTimes::new(startup.manifest(), 7).unwrap(),
        7,
        CAP,
    )
    .unwrap();
    let original_geometry = ReviewGeometry::startup(original.probe_plan()).unwrap();
    let altered_geometry = ReviewGeometry::startup(probe).unwrap();
    let policies = policies();
    let original = AdmittedProfile::new(
        &original_geometry,
        input(&original_geometry, &policies),
        caps(usize::MAX),
    )
    .unwrap();
    let altered = AdmittedProfile::new(
        &altered_geometry,
        input(&altered_geometry, &policies),
        caps(usize::MAX),
    )
    .unwrap();
    assert_ne!(original.family_identity(), altered.family_identity());
    assert_ne!(original.probe_identity(), altered.probe_identity());
    assert_ne!(original.identity(), altered.identity());
    let mut original_bytes = vec![0; original.bounds().protocol_bytes];
    let mut altered_bytes = vec![0; altered.bounds().protocol_bytes];
    original.write_canonical(&mut original_bytes).unwrap();
    altered.write_canonical(&mut altered_bytes).unwrap();
    assert_ne!(original_bytes, altered_bytes);
    let mut wrong_source = input(&altered_geometry, &policies);
    wrong_source.family = original_geometry.family_identity();
    wrong_source.probe = original_geometry.probe_identity();
    assert_eq!(
        AdmittedProfile::new(&altered_geometry, wrong_source, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidGeometry
    );
}

#[test]
fn wrong_identity_inventory_and_every_short_cap_fail_before_review() {
    let geometry = geometry();
    let policies = policies();
    let mut wrong = input(&geometry, &policies);
    wrong.problem[0] ^= 1;
    assert_eq!(
        AdmittedProfile::new(&geometry, wrong, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidProblem
    );
    wrong = input(&geometry, &policies);
    wrong.semantics[0] ^= 1;
    assert_eq!(
        AdmittedProfile::new(&geometry, wrong, caps(usize::MAX)).unwrap_err(),
        ProfileError::InvalidSemantics
    );

    let mut reordered = policies;
    reordered.swap(0, 1);
    assert_eq!(
        AdmittedProfile::new(&geometry, input(&geometry, &reordered), caps(usize::MAX))
            .unwrap_err(),
        ProfileError::InvalidInventory
    );
    assert_eq!(
        AdmittedProfile::new(
            &geometry,
            input(&geometry, &policies[..OBSERVABLE_COUNT - 1]),
            caps(usize::MAX)
        )
        .unwrap_err(),
        ProfileError::InvalidInventory
    );
    let mut duplicate = policies;
    duplicate[1].key = duplicate[0].key;
    assert_eq!(
        AdmittedProfile::new(&geometry, input(&geometry, &duplicate), caps(usize::MAX))
            .unwrap_err(),
        ProfileError::InvalidInventory
    );
    let admitted =
        AdmittedProfile::new(&geometry, input(&geometry, &policies), caps(usize::MAX)).unwrap();
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
        assert!(AdmittedProfile::new(&geometry, input(&geometry, &policies), short).is_err());
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
