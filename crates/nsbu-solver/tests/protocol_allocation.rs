//! Isolated allocator evidence for protocol admission, hashing, serialization and review.
mod protocol_support;
use nsbu_solver::verification::{protocol::FrozenProtocol, review::ReviewStatus};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    let fixture = protocol_support::Fixture::new();
    let mut bytes = [0xa5; 1550];
    let mut short = [0xa5; 1546];
    let region = Region::new(GLOBAL);
    assert!(FrozenProtocol::new(fixture.inputs(), 1546).is_err());
    let protocol = FrozenProtocol::new(fixture.inputs(), 1547).unwrap();
    assert_eq!(protocol.write_canonical(&mut bytes).unwrap(), 1547);
    assert_eq!(&bytes[1547..], &[0xa5; 3]);
    assert!(protocol.write_canonical(&mut short).is_err());
    assert_eq!(short, [0xa5; 1546]);
    assert!(protocol.review(3).is_err());
    assert_eq!(
        protocol.review(4).unwrap().status(),
        ReviewStatus::Incomplete
    );
    let measured = region.change();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.reallocations, 0);
    assert_eq!(measured.deallocations, 0);
    println!("Protocol admission, refusal, hashing, canonical output and review allocate zero bytes after caller fixture construction.");
}
