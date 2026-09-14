//! Regression coverage for captured-snapshot decoding refusals that must hold
//! before offline N512 observation: the writer in
//! `evidence/p10/avx-scheduled-endpoint/harness/src/artifact.rs` seals the
//! coefficient trailer over the payload only and the manifest binds the whole
//! file, so a re-sealed artifact must still be refused by the inner clock,
//! file-hash and identity invariants.

use super::{fields, manifest, root, write};
use crate::decode;
use crate::model::Manifest;
use sha2::{Digest, Sha256};
use std::fs;

fn reseal(manifest: &mut Manifest, bytes: &mut Vec<u8>) {
    let payload = 12 + 8 + manifest.identity.len() + 4 * 16;
    let trailer = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[payload..trailer]);
    bytes[trailer..].copy_from_slice(&digest);
    manifest.coefficient_sha256 = format!("{digest:x}");
    manifest.file_sha256 = format!("{:x}", Sha256::digest(&bytes[..]));
    fs::write(&manifest.snapshot, bytes).unwrap();
}

#[test]
fn resealed_clock_word_drift_is_refused_for_every_clock_field() {
    let root = root("decoder-clock");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    let baseline = decode::load(&item).unwrap();
    assert_eq!(baseline.clock.elapsed, item.elapsed);

    let clock_base = 12 + 8 + item.identity.len();
    let clean = fs::read(&item.snapshot).unwrap();
    for word in 0..4 {
        let mut current = item.clone();
        let mut bytes = clean.clone();
        let span = clock_base + 16 * word..clock_base + 16 * (word + 1);
        let mut value = u128::from_le_bytes(bytes[span.clone()].try_into().unwrap());
        value += 1;
        bytes[span].copy_from_slice(&value.to_le_bytes());
        reseal(&mut current, &mut bytes);
        let error = decode::load(&current).unwrap_err();
        assert!(
            error.contains("snapshot clock mismatch"),
            "clock word {word} escaped with {error}"
        );
    }
    fs::write(&item.snapshot, &clean).unwrap();
    assert!(decode::load(&item).is_ok());
}

#[test]
fn resealed_trailer_still_requires_the_reviewed_file_hash_binding() {
    let root = root("decoder-file-hash");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    let original_coefficient = item.coefficient_sha256.clone();
    assert!(decode::load(&item).is_ok());

    let payload = 12 + 8 + item.identity.len() + 4 * 16;
    let mut bytes = fs::read(&item.snapshot).unwrap();
    bytes[payload] ^= 1;
    let trailer = bytes.len() - 32;
    let digest = Sha256::digest(&bytes[payload..trailer]);
    bytes[trailer..].copy_from_slice(&digest);
    item.coefficient_sha256 = format!("{digest:x}");
    fs::write(&item.snapshot, &bytes).unwrap();
    let error = decode::load(&item).unwrap_err();
    assert!(
        error.contains("snapshot hash does not match reviewed manifest"),
        "stale file hash escaped with {error}"
    );

    item.file_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let tampered = decode::load(&item).unwrap();
    assert_ne!(tampered.coefficient_sha256, original_coefficient);
    assert_eq!(tampered.coefficient_sha256, item.coefficient_sha256);
    assert_eq!(tampered.file_sha256, item.file_sha256);
    assert_eq!(
        tampered.coefficients[0][0].re.to_bits(),
        u64::from_le_bytes(bytes[payload..payload + 8].try_into().unwrap()),
        "the re-bound artifact decodes to the tampered coefficient"
    );
    assert_ne!(tampered.coefficients[0][0], values[0][0]);
}

#[test]
fn resealed_same_length_identity_tamper_is_refused_by_the_identity_invariant() {
    let root = root("decoder-identity");
    let mut item = manifest(root.join("state.bin"), 4, "fixture");
    let values = fields(item.domain().unwrap().layout(), 1.0);
    write(&mut item, &values);
    assert!(decode::load(&item).is_ok());

    let identity_base = 12 + 8;
    let identity_end = identity_base + item.identity.len();
    assert_eq!(item.identity.len(), b"invalid".len());
    let mut bytes = fs::read(&item.snapshot).unwrap();
    assert_eq!(&bytes[identity_base..identity_end], b"fixture");
    bytes[identity_base..identity_end].copy_from_slice(b"invalid");
    reseal(&mut item, &mut bytes);
    let error = decode::load(&item).unwrap_err();
    assert!(
        error.contains("snapshot identity mismatch"),
        "re-sealed identity tamper escaped with {error}"
    );
}
