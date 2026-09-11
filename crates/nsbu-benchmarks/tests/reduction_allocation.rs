//! Isolated complete audit storage accounting; diagnostic reduction is not an integration attempt.
#[path = "../examples/reduction_support/mod.rs"]
mod reduction_support;
use reduction_support::{
    input::{Packet, Plan},
    output, reduce, CAP,
};
use stats_alloc::Region;
#[global_allocator]
static GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;
fn main() {
    let data = include_bytes!("../data/reduction-n4.bin");
    let admission = Region::new(GLOBAL);
    let plan = Plan::new(4, CAP).unwrap();
    assert!(Packet::read(&mut &data[..], 1).is_err());
    let stats = admission.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    let run = Region::new(GLOBAL);
    let packet = Packet::read(&mut &data[..], CAP).unwrap();
    let groups = reduce::all(&packet).unwrap();
    let stats = run.change();
    assert_eq!(stats.allocations, 13);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(
        stats.bytes_allocated,
        plan.packet_bytes + 12 * plan.points * 8
    );
    assert!(stats.bytes_allocated <= plan.reserved_bytes);
    let publication = Region::new(GLOBAL);
    output::write(&mut std::io::sink(), &packet, &groups).unwrap();
    let stats = publication.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
}
