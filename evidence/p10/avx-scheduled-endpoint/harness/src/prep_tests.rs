use super::*;
#[test]
fn selected_profile_is_ready_before_any_output_path_is_supplied() {
    assert_eq!(config::require_execution_ready(), Ok(()));
}
