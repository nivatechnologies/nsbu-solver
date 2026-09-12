use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyPlan, FamilySettings},
    v2_run::archive,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Layout, SpectralState, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const CAP: usize = 4 * 1024 * 1024 * 1024;
const PAIRS: [(usize, usize); 5] = [(0, 1), (1, 2), (3, 4), (4, 2), (2, 5)];

fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("checkpoint directory required");
    let times = [clock(0), clock(2048), clock(4096)];
    let family = FamilyPlan::new(
        FamilySettings {
            grids: [12, 16, 24],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([24; 3]).unwrap(),
                workers: 12,
            },
            endpoint: 4096,
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(&times, times.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let mut maximum_pair_reservation = 0;
    for (a, b) in PAIRS {
        let pa = family.branch_plan(a).unwrap();
        let pb = family.branch_plan(b).unwrap();
        let pair =
            archive::read_reservation(pa, pa.settings().configuration.limits.maximum_attempts)
                .unwrap()
                .checked_add(
                    archive::read_reservation(
                        pb,
                        pb.settings().configuration.limits.maximum_attempts,
                    )
                    .unwrap(),
                )
                .and_then(|n| n.checked_add(archive::maximum_encoded_len(pa).ok()?))
                .and_then(|n| n.checked_add(archive::maximum_encoded_len(pb).ok()?))
                .expect("reservation overflow");
        maximum_pair_reservation = maximum_pair_reservation.max(pair);
    }
    println!(
        "source={} maximum_pair_reservation={} cap={} mode=archive_only",
        env!("PILOT_SOURCE"),
        maximum_pair_reservation,
        CAP
    );
    if maximum_pair_reservation > CAP {
        panic!("cap exceeded");
    }
    for (pair_index, (a, b)) in PAIRS.into_iter().enumerate() {
        let left = read(&root, family, a);
        let right = read(&root, family, b);
        if left.state().clock() != clock(4096) || right.state().clock() != clock(4096) {
            panic!("clock mismatch");
        }
        let comparison =
            ComparisonPlan::new(left.state().plan().domain(), right.state().plan().domain())
                .unwrap()
                .compare(
                    [
                        left.state().component(0).unwrap(),
                        left.state().component(1).unwrap(),
                        left.state().component(2).unwrap(),
                    ],
                    [
                        right.state().component(0).unwrap(),
                        right.state().component(1).unwrap(),
                        right.state().component(2).unwrap(),
                    ],
                )
                .unwrap();
        println!("pair_index={pair_index} branches={a},{b} left_hash={} right_hash={} comparison={comparison:?}", hex(hash(left.state())), hex(hash(right.state())));
    }
    println!("terminal=archive-recovery-complete pairs=5 clock=4096");
}

fn read(
    root: &str,
    family: FamilyPlan<'_>,
    index: usize,
) -> nsbu_benchmarks::v2_run::archive::ImportedV2Run {
    let path = Path::new(root).join(format!("accepted-event-5-clock4096-branch{index}.bin"));
    let bytes = fs::read(path).unwrap();
    let plan = family.branch_plan(index).unwrap();
    archive::read(
        &bytes,
        plan,
        archive::maximum_encoded_len(plan).unwrap(),
        CAP,
    )
    .unwrap()
}

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn hash(state: &SpectralState) -> [u8; 32] {
    let mut h = Sha256::new();
    for axis in 0..3 {
        for z in state.component(axis).unwrap() {
            h.update(z.re.to_bits().to_le_bytes());
            h.update(z.im.to_bits().to_le_bytes());
        }
    }
    h.finalize().into()
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
