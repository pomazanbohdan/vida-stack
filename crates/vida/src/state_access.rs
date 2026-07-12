use serde_json::json;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateAccessErrorKind {
    LockContention,
    OpenFailed,
}

impl StateAccessErrorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LockContention => "lock_contention",
            Self::OpenFailed => "open_failed",
        }
    }

    pub(crate) fn blocker_code(self) -> taskflow_contracts::BlockerCode {
        match self {
            Self::LockContention => taskflow_contracts::BlockerCode::AuthoritativeStateStoreLocked,
            Self::OpenFailed => taskflow_contracts::BlockerCode::AuthoritativeStateStoreOpenFailed,
        }
    }
}

pub(crate) fn io_error_is_lock_contention(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    ) || error.raw_os_error().is_some_and(|code| {
        code == libc::EWOULDBLOCK
            || code == libc::EAGAIN
            || (cfg!(windows) && matches!(code, 5 | 32 | 33))
    })
}

pub(crate) fn message_is_lock_contention(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("lock")
        || normalized.contains("resource temporarily unavailable")
        || normalized.contains("os error 32")
        || normalized.contains("os error 33")
        || normalized.contains("os error 5")
        || normalized.contains("access is denied")
        || normalized.contains("being used by another process")
        || normalized.contains("process cannot access the file")
        || normalized.contains("portion of the file")
}

pub(crate) fn classify_state_access_error(error: &str) -> StateAccessErrorKind {
    if message_is_lock_contention(error) {
        StateAccessErrorKind::LockContention
    } else {
        StateAccessErrorKind::OpenFailed
    }
}

pub(crate) fn state_access_blocker_code(error: &str) -> &'static str {
    classify_state_access_error(error).blocker_code().as_str()
}

pub(crate) fn lock_diagnostics(state_root: &Path) -> serde_json::Value {
    let lock_path = state_root.join("LOCK");
    let metadata = std::fs::symlink_metadata(&lock_path).ok();
    let lock_is_symlink = metadata
        .as_ref()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    let modified_unix_seconds = metadata
        .as_ref()
        .filter(|_| !lock_is_symlink)
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());

    json!({
        "lock_path": lock_path,
        "lock_exists": metadata.is_some(),
        "lock_is_symlink": lock_is_symlink,
        "lock_file_size": metadata
            .as_ref()
            .filter(|_| !lock_is_symlink)
            .map(std::fs::Metadata::len),
        "lock_modified_unix_seconds": modified_unix_seconds,
    })
}

pub(crate) fn state_access_next_actions(
    error_kind: StateAccessErrorKind,
    retry_command: &str,
) -> Vec<String> {
    match error_kind {
        StateAccessErrorKind::LockContention => vec![
            format!(
                "Wait for the authoritative VIDA state-store holder to finish, then retry `{retry_command}`."
            ),
            "Inspect read-only continuation context with `vida task ready`, `vida taskflow graph-summary`, or `vida status` while the lock is held.".to_string(),
            "If no holder exists, use the VIDA recovery/reclaim flow; avoid manual filesystem edits to datastore lock files.".to_string(),
        ],
        StateAccessErrorKind::OpenFailed => vec![
            format!(
                "Inspect the state directory and retry `{retry_command}` after state access is restored."
            ),
            "Use read-only status surfaces such as `vida status` for degraded context if available."
                .to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_lock_violation_errors_are_lock_contention() {
        for code in [5, 32, 33] {
            let error = std::io::Error::from_raw_os_error(code);
            assert!(
                io_error_is_lock_contention(&error),
                "Windows raw OS error {code} should classify as lock contention"
            );
        }
    }

    #[test]
    fn db_wrapped_lock_messages_are_lock_contention() {
        for message in [
            "IO error: Access is denied. (os error 5)",
            "IO error: The process cannot access the file because another process has locked a portion of the file. (os error 33)",
            "IO error: The process cannot access the file because it is being used by another process. (os error 32)",
            "surrealkv: failed to open database: resource temporarily unavailable",
            "timed out while waiting for authoritative datastore lock",
        ] {
            assert_eq!(
                classify_state_access_error(message),
                StateAccessErrorKind::LockContention,
                "message should classify as lock contention: {message}"
            );
            assert_eq!(
                state_access_blocker_code(message),
                "authoritative_state_store_locked"
            );
        }
    }

    #[test]
    fn non_lock_messages_are_open_failed() {
        let message = "state directory is missing";
        assert_eq!(
            classify_state_access_error(message),
            StateAccessErrorKind::OpenFailed
        );
        assert_eq!(
            state_access_blocker_code(message),
            "authoritative_state_store_open_failed"
        );
    }

    #[test]
    fn lock_contention_actions_do_not_suggest_manual_deletion() {
        let actions = state_access_next_actions(
            StateAccessErrorKind::LockContention,
            "vida taskflow consume continue",
        );
        let joined = actions.join("\n").to_ascii_lowercase();
        assert!(joined.contains("recovery/reclaim"));
        assert!(!joined.contains("delete"));
        assert!(!joined.contains("remove"));
    }
}
