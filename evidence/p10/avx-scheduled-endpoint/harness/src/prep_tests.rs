use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn pending_profile_refuses_before_creating_output() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output = std::env::temp_dir().join(format!(
        "p10-n384-prep-refusal-{}-{nonce}",
        std::process::id()
    ));
    assert!(matches!(
        start(&output),
        Err(HarnessError::Numerical(SolverError::InvalidPayload))
    ));
    assert!(!output.exists());
}
