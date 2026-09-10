//! Provenance declarations retain ancestry without conferring numerical acceptance.
use nsbu_solver::{
    domain::TickClock,
    lineage::{Digest, LineageError, Origin, Profile, RecordId, Registry},
};

fn digest(byte: u8) -> Digest {
    Digest::new([byte; 32]).unwrap()
}
fn profile() -> Profile {
    Profile {
        problem: digest(1),
        execution: digest(2),
        force: digest(3),
    }
}
fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-10, 100, elapsed, 100 - elapsed).unwrap()
}

fn initial_history(registry: &mut Registry<'_>) -> [RecordId; 2] {
    let root = registry
        .append(profile(), clock(0), Origin::FromRest)
        .unwrap();
    let first = registry
        .append(profile(), clock(4), Origin::Continued(root))
        .unwrap();
    [root, first]
}

#[test]
fn identity_and_storage_are_explicit() {
    assert_eq!(Digest::new([0; 32]), Err(LineageError::MissingIdentity));
    assert_eq!(digest(9).bytes(), [9; 32]);
    assert!(profile().same_problem(Profile {
        execution: digest(9),
        force: digest(8),
        ..profile()
    }));
    assert!(!profile().same_problem(Profile {
        problem: digest(9),
        ..profile()
    }));
    assert_eq!(Registry::reservation(0), Ok(0));
    let unit = Registry::reservation(1).unwrap();
    assert!(unit > 0);
    assert_eq!(Registry::reservation(3), Ok(3 * unit));
    assert_eq!(
        Registry::reservation(usize::MAX),
        Err(LineageError::CapacityExceeded)
    );
    assert_eq!(
        Registry::reservation(isize::MAX as usize / unit + 1),
        Err(LineageError::CapacityExceeded)
    );
    let mut empty = [];
    let mut registry = Registry::new(digest(7), &mut empty, 2).unwrap();
    assert_eq!(
        registry.append(profile(), clock(0), Origin::FromRest),
        Err(LineageError::CapacityExceeded)
    );
    assert_eq!(registry.attempts_left(), 1);
    let mut storage = [None; 1];
    let id = Registry::new(digest(7), &mut storage, 1)
        .unwrap()
        .append(profile(), clock(0), Origin::FromRest)
        .unwrap();
    assert!(matches!(
        Registry::new(digest(7), &mut storage, 1),
        Err(LineageError::OccupiedStorage)
    ));
    assert_eq!(registry.get(id), Err(LineageError::UnknownParent));
}

#[test]
fn direct_continuations_retain_complete_ancestry() {
    let mut storage = [None; 4];
    let mut registry = Registry::new(digest(7), &mut storage, 4).unwrap();
    let [root, child] = initial_history(&mut registry);
    let p = Profile {
        force: digest(8),
        ..profile()
    };
    let grandchild = registry
        .append(p, clock(8), Origin::Continued(child))
        .unwrap();
    let saved = registry.get(grandchild).unwrap();
    assert_eq!(saved.profile(), p);
    assert_eq!(saved.clock(), clock(8));
    assert_eq!(saved.origin(), Origin::Continued(child));
    assert!(saved.is_valid());
    assert!(saved.is_direct());
    assert_eq!(registry.get(root).unwrap().origin(), Origin::FromRest);
    assert_eq!(registry.get(child).unwrap().clock(), clock(4));
    assert_eq!(registry.attempts_left(), 1);
}

#[test]
fn transfers_and_reference_seeds_never_become_direct_branches() {
    let mut storage = [None; 6];
    let mut registry = Registry::new(digest(7), &mut storage, 6).unwrap();
    let root = registry
        .append(profile(), clock(0), Origin::FromRest)
        .unwrap();
    let coarse = registry
        .append(profile(), clock(4), Origin::Continued(root))
        .unwrap();
    let fine = Profile {
        execution: digest(5),
        ..profile()
    };
    let origin = Origin::Transferred {
        parent: coarse,
        errors: digest(6),
    };
    let transferred = registry.append(fine, clock(4), origin).unwrap();
    assert_eq!(registry.get(transferred).unwrap().origin(), origin);
    assert!(!registry.get(transferred).unwrap().is_direct());
    let continued = registry
        .append(fine, clock(8), Origin::Continued(transferred))
        .unwrap();
    assert!(!registry.get(continued).unwrap().is_direct());
    let local = registry
        .append(profile(), clock(8), Origin::LocalReference)
        .unwrap();
    assert!(!registry.get(local).unwrap().is_direct());
    let descendant = registry
        .append(profile(), clock(12), Origin::Continued(local))
        .unwrap();
    assert!(!registry.get(descendant).unwrap().is_direct());
    assert_eq!(registry.attempts_left(), 0);
    assert_eq!(
        registry.append(profile(), clock(16), Origin::Continued(descendant)),
        Err(LineageError::CapacityExceeded)
    );
}

#[test]
fn malformed_origins_spend_attempts_without_publishing_records() {
    let mut storage = [None; 4];
    let mut registry = Registry::new(digest(7), &mut storage, 20).unwrap();
    assert_eq!(
        registry.append(profile(), clock(4), Origin::FromRest),
        Err(LineageError::InvalidClock)
    );
    let [_, child] = initial_history(&mut registry);
    let before = registry.get(child).unwrap();
    for bad in [
        clock(0),
        clock(4),
        TickClock::restore(-9, 100, 8, 92).unwrap(),
        TickClock::restore(-10, 104, 8, 96).unwrap(),
    ] {
        assert_eq!(
            registry.append(profile(), bad, Origin::Continued(child)),
            Err(LineageError::InvalidClock)
        );
    }
    assert_eq!(
        registry.append(
            Profile {
                problem: digest(8),
                ..profile()
            },
            clock(8),
            Origin::Continued(child)
        ),
        Err(LineageError::DifferentProblem)
    );
    assert_eq!(
        registry.append(
            Profile {
                execution: digest(8),
                ..profile()
            },
            clock(8),
            Origin::Continued(child)
        ),
        Err(LineageError::DifferentProfile)
    );
    assert_eq!(
        registry.append(
            profile(),
            clock(8),
            Origin::Transferred {
                parent: child,
                errors: digest(8)
            }
        ),
        Err(LineageError::InvalidClock)
    );
    let mut foreign_storage = [None; 1];
    let foreign = Registry::new(digest(8), &mut foreign_storage, 1)
        .unwrap()
        .append(profile(), clock(0), Origin::FromRest)
        .unwrap();
    assert_eq!(
        registry.append(profile(), clock(8), Origin::Continued(foreign)),
        Err(LineageError::UnknownParent)
    );
    assert_eq!(registry.get(child).unwrap(), before);
    assert_eq!(registry.attempts_left(), 9);
    registry
        .append(profile(), clock(8), Origin::Continued(child))
        .unwrap();
}

#[test]
fn force_invalidation_replays_transitively_and_never_revives_history() {
    let mut storage = [None; 6];
    let mut registry = Registry::new(digest(7), &mut storage, 8).unwrap();
    let root = registry
        .append(profile(), clock(0), Origin::FromRest)
        .unwrap();
    let changed = Profile {
        force: digest(8),
        ..profile()
    };
    let child = registry
        .append(changed, clock(4), Origin::Continued(root))
        .unwrap();
    let transfer = registry
        .append(
            changed,
            clock(4),
            Origin::Transferred {
                parent: child,
                errors: digest(6),
            },
        )
        .unwrap();
    let grandchild = registry
        .append(changed, clock(8), Origin::Continued(transfer))
        .unwrap();
    let independent = registry
        .append(changed, clock(0), Origin::FromRest)
        .unwrap();
    assert_eq!(
        registry.invalidate_force(profile().force, 4),
        Err(LineageError::CapacityExceeded)
    );
    assert!(registry.get(root).unwrap().is_valid());
    assert!(registry.get(grandchild).unwrap().is_valid());
    assert_eq!(registry.invalidate_force(digest(9), 5), Ok(0));
    assert_eq!(registry.invalidate_force(profile().force, 5), Ok(4));
    for id in [root, child, transfer, grandchild] {
        assert!(!registry.get(id).unwrap().is_valid());
    }
    assert!(registry.get(independent).unwrap().is_valid());
    assert_eq!(registry.invalidate_force(profile().force, 5), Ok(0));
    assert_eq!(
        registry.append(changed, clock(12), Origin::Continued(grandchild)),
        Err(LineageError::InvalidatedParent)
    );
    assert_eq!(registry.invalidate_force(changed.force, 5), Ok(1));
    assert!(!registry.get(independent).unwrap().is_valid());
    assert_eq!(registry.invalidate_force(changed.force, 5), Ok(0));
}
