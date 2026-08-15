//! Consume continuation policy extracted from the VIDA shell adapter.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateAccessErrorKind {
    LockContention,
    OpenFailed,
}

impl StateAccessErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LockContention => "lock_contention",
            Self::OpenFailed => "open_failed",
        }
    }

    pub fn blocker_code(self) -> &'static str {
        match self {
            Self::LockContention => "authoritative_state_store_locked",
            Self::OpenFailed => "authoritative_state_store_open_failed",
        }
    }
}

pub fn classify_state_access_error(error: &str) -> StateAccessErrorKind {
    if error.to_ascii_lowercase().contains("lock") {
        StateAccessErrorKind::LockContention
    } else {
        StateAccessErrorKind::OpenFailed
    }
}

pub fn state_access_blocker_code(error: &str) -> &'static str {
    classify_state_access_error(error).blocker_code()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeferredAgentHandoffInput<'a> {
    pub surface_name: &'a str,
    pub dispatch_kind: &'a str,
    pub dispatch_status: &'a str,
    pub downstream_dispatch_ready: bool,
    pub downstream_dispatch_packet_path: Option<&'a str>,
}

pub fn should_defer_agent_handoff(input: DeferredAgentHandoffInput<'_>) -> bool {
    input.surface_name == "vida taskflow consume continue"
        && input.dispatch_kind == "agent_lane"
        && (input.dispatch_status == "routed"
            || (input.downstream_dispatch_ready
                && input
                    .downstream_dispatch_packet_path
                    .is_some_and(|path| !path.trim().is_empty())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_access_error_classification_distinguishes_locks_from_open_failures() {
        assert_eq!(
            classify_state_access_error("database LOCK is held"),
            StateAccessErrorKind::LockContention
        );
        assert_eq!(
            state_access_blocker_code("database LOCK is held"),
            "authoritative_state_store_locked"
        );
        assert_eq!(
            classify_state_access_error("file not found"),
            StateAccessErrorKind::OpenFailed
        );
        assert_eq!(
            state_access_blocker_code("file not found"),
            "authoritative_state_store_open_failed"
        );
    }

    #[test]
    fn defer_agent_handoff_only_for_consume_continue_agent_lane_work() {
        assert!(should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume continue",
            dispatch_kind: "agent_lane",
            dispatch_status: "routed",
            downstream_dispatch_ready: false,
            downstream_dispatch_packet_path: None,
        }));

        assert!(should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume continue",
            dispatch_kind: "agent_lane",
            dispatch_status: "executed",
            downstream_dispatch_ready: true,
            downstream_dispatch_packet_path: Some("packet.json"),
        }));
        assert!(!should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume continue",
            dispatch_kind: "agent_lane",
            dispatch_status: "executed",
            downstream_dispatch_ready: true,
            downstream_dispatch_packet_path: None,
        }));
        assert!(!should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume continue",
            dispatch_kind: "agent_lane",
            dispatch_status: "executed",
            downstream_dispatch_ready: false,
            downstream_dispatch_packet_path: Some("packet.json"),
        }));

        assert!(!should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume resume",
            dispatch_kind: "agent_lane",
            dispatch_status: "routed",
            downstream_dispatch_ready: false,
            downstream_dispatch_packet_path: None,
        }));

        assert!(!should_defer_agent_handoff(DeferredAgentHandoffInput {
            surface_name: "vida taskflow consume continue",
            dispatch_kind: "manual",
            dispatch_status: "routed",
            downstream_dispatch_ready: true,
            downstream_dispatch_packet_path: Some("packet.json"),
        }));
    }
}
