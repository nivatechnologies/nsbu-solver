//! Canonical policy identity, every numerical parameter, exact geometry and bounded review.
mod protocol_support;
use nsbu_solver::verification::{
    budget::{Budget, CHANNELS},
    protocol::{reservation, FrozenProtocol},
    refinement::{Requirement, Rule},
    review::ReviewStatus,
    times::TestedTimes,
    VerificationError,
};
use protocol_support::{clock, probes, Fixture};

fn identity(fixture: &Fixture) -> [u8; 32] {
    FrozenProtocol::new(fixture.inputs(), 1 << 20)
        .unwrap()
        .identity()
}

#[test]
fn canonical_fingerprint_and_bounded_review_match_the_declared_schema() {
    let fixture = Fixture::new();
    let protocol = FrozenProtocol::new(fixture.inputs(), 1547).unwrap();
    assert_eq!(protocol.encoded_bytes(), 1547);
    assert_eq!(reservation(fixture.inputs()).unwrap(), 1547);
    // Independently encoded with Python struct/int bytes and exact Fraction rounding
    // for the channel-allocation sum; all eleven fixture requirements are Refinement.
    assert_eq!(
        protocol.identity(),
        [
            0x72, 0xc4, 0x00, 0x33, 0x2f, 0xd6, 0x63, 0x3c, 0x30, 0x7f, 0xaa, 0x97, 0x2a, 0x61,
            0x42, 0x20, 0xe3, 0x3a, 0x65, 0x32, 0x41, 0xbf, 0xd5, 0x1f, 0x62, 0x88, 0x34, 0x85,
            0x11, 0xcf, 0x73, 0x54,
        ]
    );
    assert_eq!(protocol.inputs().policies.as_slice()[0].key, 41);
    let review = protocol.review(7).unwrap();
    assert_eq!(review.status(), ReviewStatus::Incomplete);
    assert_eq!(review.progress().required, 4);
    assert_eq!(review.progress().attempts_left, 7);
    assert_eq!(review.next(), Some((clock(0), 41)));
    assert!(protocol.review(3).is_err());
}

#[test]
fn cap_and_missing_identity_fail_before_a_protocol_is_issued() {
    let fixture = Fixture::new();
    assert!(matches!(
        FrozenProtocol::new(fixture.inputs(), 1546),
        Err(VerificationError::CapacityExceeded)
    ));
    let mut input = fixture.inputs();
    input.problem = [0; 32];
    assert!(FrozenProtocol::new(input, 1547).is_err());
    input = fixture.inputs();
    input.semantics = [0; 32];
    assert!(FrozenProtocol::new(input, 1547).is_err());
    input = fixture.inputs();
    input.time_sets[1] = input.time_sets[0];
    assert!(FrozenProtocol::new(input, 1547).is_err());
}

#[test]
fn identities_keys_order_counts_and_total_tolerances_are_bound() {
    let mut fixture = Fixture::new();
    let baseline = identity(&fixture);
    for problem in [true, false] {
        let mut input = fixture.inputs();
        if problem {
            input.problem = [3; 32];
        } else {
            input.semantics = [3; 32];
        }
        assert_ne!(
            FrozenProtocol::new(input, 1 << 20).unwrap().identity(),
            baseline
        );
    }
    fixture.policies[0].key = 42;
    assert_ne!(identity(&fixture), baseline);
    fixture.policies[0].key = 41;
    let rules = CHANNELS.map(|channel| fixture.policies[0].budget.rule(channel));
    fixture.policies[0].budget = Budget::new(2.0, rules).unwrap();
    assert_ne!(identity(&fixture), baseline);
    let mut other = fixture.policies[0];
    other.key = 43;
    fixture.policies.push(other);
    let forward = identity(&fixture);
    fixture.policies.reverse();
    assert_ne!(identity(&fixture), forward);
}

#[test]
fn every_channel_budget_reduction_and_floor_is_bound() {
    let original = Fixture::new();
    let baseline = identity(&original);
    for channel in CHANNELS {
        for (budget, reduction, floor) in [(0.02, 0.5, 0.1), (0.01, 0.4, 0.1), (0.01, 0.5, 0.2)] {
            let mut fixture = Fixture::new();
            let mut rules = CHANNELS.map(|ch| fixture.policies[0].budget.rule(ch));
            rules[channel as usize] =
                Rule::new(budget, reduction, floor, Requirement::Refinement).unwrap();
            fixture.policies[0].budget = Budget::new(1.0, rules).unwrap();
            assert_ne!(identity(&fixture), baseline);
        }
    }
    let mut fixture = Fixture::new();
    let mut rules = CHANNELS.map(|ch| fixture.policies[0].budget.rule(ch));
    rules[2] = Rule::new(0.01, 0.5, 0.1, Requirement::Sensitivity).unwrap();
    fixture.policies[0].budget = Budget::new(1.0, rules).unwrap();
    assert_ne!(identity(&fixture), baseline);
}

#[test]
fn all_time_levels_and_reconstruction_geometry_are_bound() {
    let mut fixture = Fixture::new();
    let baseline = identity(&fixture);
    fixture.times[2].insert(1, clock(32));
    assert_ne!(identity(&fixture), baseline);
    let with_fine = identity(&fixture);
    fixture.times[1].insert(1, clock(32));
    assert_ne!(identity(&fixture), with_fine);
    let with_middle = identity(&fixture);
    fixture.times[0].insert(1, clock(64));
    assert_ne!(identity(&fixture), with_middle);
    let mut fixture = Fixture::new();
    fixture.probes[0] = probes([112, 120, 128], 127);
    assert_ne!(identity(&fixture), baseline);
    fixture.probes[0] = probes([96, 112, 128], 125);
    fixture.times[2][2] = clock(125);
    assert_ne!(identity(&fixture), baseline);
}

#[test]
fn reconstruction_cannot_be_rebound_to_a_different_fine_manifest() {
    let fixture = Fixture::new();
    let finer = [clock(0), clock(32), clock(64), clock(127), clock(128)];
    let mut input = fixture.inputs();
    input.time_sets[2] = TestedTimes::new(&finer, 5).unwrap();
    assert!(matches!(
        FrozenProtocol::new(input, 1 << 20),
        Err(VerificationError::InvalidTimes)
    ));
}

#[test]
fn canonical_bytes_match_the_fingerprint_and_preserve_short_or_trailing_storage() {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new();
    let protocol = FrozenProtocol::new(fixture.inputs(), 1547).unwrap();
    let mut bytes = [0xa5; 1550];
    assert_eq!(protocol.write_canonical(&mut bytes).unwrap(), 1547);
    assert_eq!(
        Sha256::digest(&bytes[..1547]).as_slice(),
        protocol.identity()
    );
    assert_eq!(&bytes[1547..], &[0xa5; 3]);
    assert_eq!(&bytes[..16], b"NSBUPROTOCOL0001");
    let mut short = [0xa5; 1546];
    assert!(protocol.write_canonical(&mut short).is_err());
    assert_eq!(short, [0xa5; 1546]);
}
