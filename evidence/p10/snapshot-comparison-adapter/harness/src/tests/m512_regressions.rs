use super::*;

const CONTRACT_MISMATCH: &str = "matched M512 spatial diagnostic contract mismatch";

fn fixtures(name: &str) -> (PathBuf, Manifest, Manifest) {
    let root = root(name);
    let mut left = manifest(root.join("left.bin"), 4, "left");
    let mut right = manifest(root.join("right.bin"), 6, "right");
    enable_matched_m512_spatial(&mut left, &mut right);
    (root, left, right)
}

fn assert_refused(left: &Manifest, right: &Manifest) {
    assert_eq!(
        compare::validate_manifest_pair(left, right).unwrap_err(),
        CONTRACT_MISMATCH
    );
}

fn identity_field<'a>(identity: &'a str, key: &str) -> Option<&'a str> {
    identity
        .split(';')
        .filter_map(|field| field.strip_prefix(&format!("{key}=")))
        .next()
}

#[test]
fn matched_m512_rejects_reversed_pair_direction() {
    let (_root, left, right) = fixtures("m512-reverse");
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());
    assert_refused(&right, &left);

    let mut left = left.clone();
    let mut right = right.clone();
    std::mem::swap(&mut left.source_commit, &mut right.source_commit);
    std::mem::swap(&mut left.plan_sha256, &mut right.plan_sha256);
    std::mem::swap(&mut left.identity, &mut right.identity);
    assert_eq!(left.dimensions, [384; 3]);
    assert_eq!(right.dimensions, [512; 3]);
    assert_refused(&left, &right);
}

#[test]
fn matched_m512_rejects_duplicate_required_identity_keys() {
    let (_root, mut left, right) = fixtures("m512-duplicate-keys");
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    left.identity = format!(
        "{};source={}",
        left.identity,
        identity_field(&left.identity, "source").unwrap()
    );
    assert_eq!(
        left.identity
            .split(';')
            .filter(|field| field.starts_with("source="))
            .count(),
        2
    );
    assert_refused(&left, &right);

    let (_root, left, mut right) = fixtures("m512-duplicate-keys-right");
    let repeated = identity_field(&right.identity, "test_source")
        .unwrap()
        .to_owned();
    right.identity = format!("{};test_source={}", right.identity, repeated);
    assert_eq!(
        right
            .identity
            .split(';')
            .filter(|field| field.starts_with("test_source="))
            .count(),
        2
    );
    assert_refused(&left, &right);
}

#[test]
fn matched_m512_rejects_removed_required_right_identity_fields() {
    let (_root, left, right) = fixtures("m512-removed-fields");
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    for dropped in [
        "test_source",
        "external_stop",
        "schema",
        "production_source",
    ] {
        let kept: Vec<&str> = right
            .identity
            .split(';')
            .filter(|field| !field.starts_with(&format!("{dropped}=")))
            .collect();
        assert_eq!(kept.len(), right.identity.split(';').count() - 1);
        let mut value = right.clone();
        value.identity = kept.join(";");
        assert!(identity_field(&value.identity, dropped).is_none());
        assert!(compare::validate_manifest_pair(&left, &value).is_err());
    }
}

#[test]
fn matched_m512_rejects_one_sided_left_evolution_changes() {
    let (_root, left, right) = fixtures("m512-left-evolution");
    assert!(compare::validate_manifest_pair(&left, &right).is_ok());

    for mutate in [
        {
            let mut value = left.clone();
            value.evolution.viscosity = 0.5;
            value
        },
        {
            let mut value = left.clone();
            value.evolution.method = "hochbruck-ostermann".into();
            value
        },
        {
            let mut value = left.clone();
            value.evolution.case_sha256 = "d".repeat(64);
            value
        },
        {
            let mut value = left.clone();
            value.evolution.schedule[0].step_ticks = 32;
            value
        },
    ] {
        assert_ne!(mutate.evolution, left.evolution);
        assert_ne!(mutate.evolution, right.evolution);
        assert_refused(&mutate, &right);
    }
}
