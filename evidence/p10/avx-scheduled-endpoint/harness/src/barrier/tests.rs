use super::*;

fn temp_dir(label: &str) -> PathBuf {
    let nonce = now_epoch();
    let path = std::env::temp_dir().join(format!(
        "p10-h32-barrier-{label}-{}-{nonce}-{}",
        std::process::id(),
        thread_seq()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

thread_local! {
    static SEQ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
fn thread_seq() -> u64 {
    SEQ.with(|seq| {
        let value = seq.get();
        seq.set(value + 1);
        value
    })
}

#[test]
fn token_derivation_is_bound_to_every_handshake_field() {
    let secret = [7_u8; 32];
    let base = derive_token(&secret, "release", &"a".repeat(64), 32, &"b".repeat(64));
    assert!(is_hex64(&base));
    assert_ne!(base, derive_token(&secret, "abort", &"a".repeat(64), 32, &"b".repeat(64)));
    assert_ne!(base, derive_token(&[8_u8; 32], "release", &"a".repeat(64), 32, &"b".repeat(64)));
    assert_ne!(base, derive_token(&secret, "release", &"c".repeat(64), 32, &"b".repeat(64)));
    assert_ne!(base, derive_token(&secret, "release", &"a".repeat(64), 64, &"b".repeat(64)));
    assert_ne!(base, derive_token(&secret, "release", &"a".repeat(64), 32, &"c".repeat(64)));
}

#[test]
fn environment_binding_refuses_partial_or_malformed_armings() {
    for key in [DIR_ENV, NONCE_ENV, SECRET_ENV, CLOCK_ENV, DEADLINE_ENV] {
        std::env::remove_var(key);
    }
    assert!(Barrier::from_env().unwrap().is_none());
    std::env::set_var(NONCE_ENV, "a".repeat(64));
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_partial_environment"))
    ));
    std::env::remove_var(NONCE_ENV);
    let dir = temp_dir("env");
    std::env::set_var(DIR_ENV, &dir);
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_nonce_missing"))
    ));
    std::env::set_var(NONCE_ENV, "zz");
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_nonce_malformed"))
    ));
    std::env::set_var(NONCE_ENV, "a".repeat(64));
    std::env::set_var(SECRET_ENV, "aa");
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_secret_malformed"))
    ));
    std::env::set_var(SECRET_ENV, "b".repeat(64));
    std::env::set_var(CLOCK_ENV, "32.5");
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_clock_malformed"))
    ));
    std::env::set_var(CLOCK_ENV, "32");
    std::env::set_var(DEADLINE_ENV, "0");
    assert!(matches!(
        Barrier::from_env(),
        Err(HarnessError::Barrier("barrier_deadline_not_future"))
    ));
    std::env::remove_var(DEADLINE_ENV);
    std::env::remove_var(DIR_ENV);
    std::env::remove_var(NONCE_ENV);
    std::env::remove_var(SECRET_ENV);
    std::env::remove_var(CLOCK_ENV);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn malformed_tokens_never_pass_and_abort_token_wins_over_release() {
    let dir = temp_dir("tokens");
    let secret = [3_u8; 32];
    let nonce = "d".repeat(64);
    let state = "e".repeat(64);
    fs::write(dir.join(RELEASE_FILE), b"not-a-token\n").unwrap();
    assert_eq!(read_token(&dir.join(RELEASE_FILE)), None);
    fs::write(dir.join(RELEASE_FILE), format!("{:0>64}", "0")).unwrap();
    assert!(read_token(&dir.join(RELEASE_FILE)).is_some());
    fs::write(dir.join(RELEASE_FILE), "f".repeat(64) + "\n\n").unwrap();
    assert_eq!(read_token(&dir.join(RELEASE_FILE)), None);
    fs::write(dir.join(RELEASE_FILE), vec![b'f'; 300]).unwrap();
    assert_eq!(read_token(&dir.join(RELEASE_FILE)), None);
    let abort = derive_token(&secret, "abort", &nonce, 32, &state);
    let release = derive_token(&secret, "release", &nonce, 32, &state);
    fs::write(dir.join(ABORT_FILE), format!("{abort}\n")).unwrap();
    fs::write(dir.join(RELEASE_FILE), format!("{release}\n")).unwrap();
    let barrier = Barrier {
        dir: dir.clone(),
        nonce,
        secret,
        armed_clock: 32,
        deadline_epoch: now_epoch() + 60,
        phase: BarrierPhase::Armed,
    };
    assert!(matches!(
        barrier.wait_for_token(&state),
        Err(HarnessError::Barrier("barrier_aborted_by_supervisor"))
    ));
    fs::remove_file(dir.join(ABORT_FILE)).unwrap();
    fs::write(dir.join(RELEASE_FILE), "wrong-value-and-padded-xx").unwrap();
    let expired = Barrier {
        deadline_epoch: now_epoch(),
        ..barrier
    };
    assert!(matches!(
        expired.wait_for_token(&state),
        Err(HarnessError::Barrier("barrier_expired"))
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn token_derivation_matches_the_python_supervisor_vector() {
    let secret = hex::decode_lossy(
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap();
    let nonce = "0a".repeat(32);
    let state = "0b".repeat(32);
    assert_eq!(
        derive_token(&secret, "release", &nonce, 32, &state),
        "cabadcab44f27de96f99372b711e0bb082b73b36be6aeec02ba19aba1e18cd98"
    );
    assert_eq!(
        derive_token(&secret, "abort", &nonce, 32, &state),
        "4c2f9f96a54e99565d589ae8a47d827692f06cc2c1caa1fcdb407804ebd8dcf5"
    );
}

#[test]
fn record_state_hash_extraction_requires_exactly_one_valid_field() {
    let good = format!("{{\n \"state_sha256\": \"{}\"\n}}\n", "a".repeat(64));
    assert_eq!(record_state_sha(&good).unwrap(), "a".repeat(64));
    assert!(record_state_sha("{}").is_err());
    assert!(record_state_sha("{\"state_sha256\": \"xyz\"}").is_err());
    let duplicated = format!("{good}{good}");
    assert!(record_state_sha(&duplicated).is_err());
}

#[test]
fn authenticated_release_completes_the_barrier_one_shot_forever() {
    let dir = temp_dir("one-shot");
    let output = temp_dir("one-shot-output");
    let secret = [5_u8; 32];
    let nonce = "9".repeat(64);
    let state = "e".repeat(64);
    let bundle = output.join("step-001-clock-0032");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(
        bundle.join("record.json"),
        format!("{{\n  \"state_sha256\": \"{state}\"\n}}\n"),
    )
    .unwrap();
    fs::write(
        dir.join(RELEASE_FILE),
        format!("{}\n", derive_token(&secret, "release", &nonce, 32, &state)),
    )
    .unwrap();
    let mut barrier = Barrier {
        dir: dir.clone(),
        nonce: nonce.clone(),
        secret,
        armed_clock: 32,
        deadline_epoch: now_epoch() + 60,
        phase: BarrierPhase::Waiting,
    };
    barrier.after_commit(&output, 1, 32).unwrap();
    assert_eq!(barrier.phase, BarrierPhase::Completed);
    let receipt = fs::read_to_string(dir.join(ARMED_RECEIPT)).unwrap();
    assert!(receipt.contains("\"clock\": 32"));

    // A valid abort token for any later commit is planted so that any park or
    // token read after completion would fail closed; completion ignores both.
    fs::write(
        dir.join(ABORT_FILE),
        format!("{}\n", derive_token(&secret, "abort", &nonce, 64, &state)),
    )
    .unwrap();
    fs::write(
        dir.join(RELEASE_FILE),
        format!("{}\n", derive_token(&secret, "release", &nonce, 64, &state)),
    )
    .unwrap();

    // A later commit at a different clock completes immediately: no re-arm,
    // no parking, no token handling.
    barrier.after_commit(&output, 2, 64).unwrap();
    // A later commit at the armed clock value also completes immediately.
    barrier.after_commit(&output, 3, 32).unwrap();
    assert_eq!(barrier.phase, BarrierPhase::Completed);

    // The single armed receipt is byte-identical and no second receipt exists.
    assert_eq!(fs::read_to_string(dir.join(ARMED_RECEIPT)).unwrap(), receipt);
    let receipts = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("barrier-armed")
        })
        .count();
    assert_eq!(receipts, 1);

    // A completed barrier ignores even a fresh armed-clock bundle without the
    // durable record; it returns Ok before touching any state.
    barrier.after_commit(&output, 4, 32).unwrap();

    // Disarming is terminal too: after expiry the hook fails closed forever.
    barrier.phase = BarrierPhase::Disarmed;
    assert!(matches!(
        barrier.after_commit(&output, 5, 32),
        Err(HarnessError::Barrier("barrier_disarmed"))
    ));
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(output).unwrap();
}
