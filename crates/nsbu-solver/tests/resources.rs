//! Conservative reservations reproduce the reviewed base before declared extras.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan};
use nsbu_solver::SolverError;

const EXTRAS: ExtraStorage = ExtraStorage {
    fft: 11,
    force: 13,
    diagnostics: 17,
    overhead: 19,
};

#[test]
fn exact_ledger_and_cap_boundary_are_checked_without_grid_allocation() {
    let domain = Domain::new([4, 8, 12], [1.0; 3], 1.0).unwrap();
    let plan = ResourcePlan::new(domain, EXTRAS, usize::MAX, Epoch(7)).unwrap();
    assert_eq!(plan.domain(), domain);
    assert_eq!(plan.epoch(), Epoch(7));
    assert_eq!(
        plan.classes(),
        [224 * 576, 1296 * 72, 720 * 48, 224 * 48, 11, 13, 17, 19]
    );
    assert_eq!(plan.total(), 267708);
    assert_eq!(
        ResourcePlan::new(domain, EXTRAS, plan.total(), Epoch(7)),
        Ok(plan)
    );
    assert_eq!(
        ResourcePlan::new(domain, EXTRAS, plan.total() - 1, Epoch(7)),
        Err(SolverError::ResourceLimit)
    );
    assert_eq!(
        ResourcePlan::new(domain, EXTRAS, 0, Epoch(7)),
        Err(SolverError::ResourceLimit)
    );
}

#[test]
fn reviewed_cubic_reservations_are_reproduced() {
    let empty = ExtraStorage {
        fft: 0,
        force: 0,
        diagnostics: 0,
        overhead: 0,
    };
    for (n, expected) in [
        (128, 1.25336),
        (256, 9.98218),
        (512, 79.67871),
        (1024, 636.71484),
    ] {
        let plan = ResourcePlan::new(
            Domain::new([n; 3], [1.0; 3], 1.0).unwrap(),
            empty,
            usize::MAX,
            Epoch(7),
        )
        .unwrap();
        let gib = plan.total() as f64 / (1_u64 << 30) as f64;
        assert!((gib - expected).abs() < 0.000005);
    }
}

#[test]
fn class_and_sum_overflows_are_refused() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    for extra in [
        ExtraStorage {
            fft: usize::MAX,
            ..EXTRAS
        },
        ExtraStorage {
            force: usize::MAX,
            ..EXTRAS
        },
        ExtraStorage {
            diagnostics: usize::MAX,
            ..EXTRAS
        },
        ExtraStorage {
            overhead: usize::MAX,
            ..EXTRAS
        },
    ] {
        assert_eq!(
            ResourcePlan::new(domain, extra, usize::MAX, Epoch(7)),
            Err(SolverError::SizeOverflow)
        );
    }
    // Large but individually addressable layout; reservation multiplication overflows.
    let large = Domain::new([4, 4, 1_usize << 53], [1.0; 3], 1.0).unwrap();
    assert_eq!(
        ResourcePlan::new(large, EXTRAS, usize::MAX, Epoch(7)),
        Err(SolverError::SizeOverflow)
    );
}
