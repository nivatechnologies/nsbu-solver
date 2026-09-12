//! Allocation isolation for frozen exact-v2 review-profile admission.
use nsbu_benchmarks::v2_experiment::{
    diagnostic::StartupProfile,
    review_profile::{
        semantics_identity, AdmittedProfile, ProfileCaps, ProfileInputs, ReviewGeometry,
        OBSERVABLES, OBSERVABLE_COUNT, PROBLEM_IDENTITY,
    },
};
use nsbu_solver::verification::{
    budget::{Budget, CHANNELS},
    policy::ObservablePolicy,
    refinement::{Requirement, Rule},
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;
const CAP: usize = 256 * 1024 * 1024;
const ROWS: usize = OBSERVABLE_COUNT * 7;
const PAIRS: usize = OBSERVABLE_COUNT * (OBSERVABLE_COUNT - 1) / 2;
const BYTES: usize = 29_396;

fn main() {
    let budget = test_only_budget();
    let policies = OBSERVABLES.map(|observable| ObservablePolicy {
        key: observable.key,
        budget,
    });
    let startup = StartupProfile::new().unwrap();
    let diagnostic = startup.plan(CAP).unwrap();
    let mut canonical = [0u8; BYTES];
    let region = Region::new(GLOBAL);
    let geometry = ReviewGeometry::startup(diagnostic.probe_plan()).unwrap();
    let admitted = AdmittedProfile::new(
        &geometry,
        ProfileInputs {
            problem: PROBLEM_IDENTITY,
            semantics: semantics_identity(),
            policies: &policies,
        },
        ProfileCaps {
            policy_pair_checks: PAIRS,
            protocol_bytes: BYTES,
            required_rows: ROWS,
            review_attempts: ROWS,
        },
    )
    .unwrap();
    assert_eq!(admitted.write_canonical(&mut canonical).unwrap(), BYTES);
    let change = region.change();
    assert_eq!(change.allocations, 0);
    assert_eq!(change.deallocations, 0);
    assert_eq!(admitted.bounds().protocol_bytes, BYTES);
    assert_eq!(admitted.bounds().required_rows, ROWS);
    eprintln!(
        "review profile protocol_bytes={} semantics_bytes={} rows={} pair_checks={} steady_allocations={}",
        admitted.bounds().protocol_bytes,
        admitted.bounds().semantics_bytes,
        admitted.bounds().required_rows,
        admitted.bounds().policy_pair_checks,
        change.allocations
    );
}

fn test_only_budget() -> Budget {
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
