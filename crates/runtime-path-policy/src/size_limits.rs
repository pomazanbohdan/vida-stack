pub const DEFAULT_JSON_ARTIFACT_MAX_BYTES: u64 = 1024 * 1024;
pub const HOST_BRIDGE_REQUEST_MAX_BYTES: u64 = 256 * 1024;
pub const HOST_BRIDGE_RESULT_MAX_BYTES: u64 = 1024 * 1024;
pub const TASK_ATTEMPT_ARTIFACT_MAX_BYTES: u64 = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_limits_preserve_expected_values_and_ordering() {
        assert_eq!(DEFAULT_JSON_ARTIFACT_MAX_BYTES, 1024 * 1024);
        assert_eq!(HOST_BRIDGE_REQUEST_MAX_BYTES, 256 * 1024);
        assert_eq!(HOST_BRIDGE_RESULT_MAX_BYTES, 1024 * 1024);
        assert_eq!(TASK_ATTEMPT_ARTIFACT_MAX_BYTES, 64 * 1024);
        assert!(TASK_ATTEMPT_ARTIFACT_MAX_BYTES < HOST_BRIDGE_REQUEST_MAX_BYTES);
        assert!(HOST_BRIDGE_REQUEST_MAX_BYTES < HOST_BRIDGE_RESULT_MAX_BYTES);
        assert_eq!(
            DEFAULT_JSON_ARTIFACT_MAX_BYTES,
            HOST_BRIDGE_RESULT_MAX_BYTES
        );
    }
}
