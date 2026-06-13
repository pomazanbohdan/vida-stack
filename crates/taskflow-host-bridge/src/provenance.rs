use serde::{Deserialize, Serialize};

use crate::request::HostBridgeRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeProvenanceInput {
    pub request: HostBridgeRequest,
    pub expected_run_id: Option<String>,
    pub expected_task_id: Option<String>,
    pub expected_dispatch_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeProvenanceDecision {
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub reason: String,
}

pub fn validate_host_bridge_request_provenance(
    input: &HostBridgeProvenanceInput,
) -> HostBridgeProvenanceDecision {
    let mut blockers = Vec::new();
    let request = &input.request;

    if request.dispatch_transport != "host_tool_bridge" {
        blockers.push("dispatch_transport_not_host_tool_bridge".to_string());
    }
    if request.receipt_mode != "host_bridge_receipt" {
        blockers.push("receipt_mode_not_host_bridge_receipt".to_string());
    }
    if !matches!(
        request.status.as_str(),
        "pending" | "pass" | "retryable_blocked"
    ) {
        blockers.push("request_status_not_admissible".to_string());
    }
    if input
        .expected_run_id
        .as_deref()
        .is_some_and(|expected| expected != request.run_id)
    {
        blockers.push("run_id_mismatch".to_string());
    }
    if input
        .expected_task_id
        .as_deref()
        .is_some_and(|expected| expected != request.task_id)
    {
        blockers.push("task_id_mismatch".to_string());
    }
    if input
        .expected_dispatch_target
        .as_deref()
        .is_some_and(|expected| expected != request.dispatch_target)
    {
        blockers.push("dispatch_target_mismatch".to_string());
    }

    decision(blockers)
}

#[must_use]
pub fn host_bridge_provenance_public_blocker_code(blocker_code: &str) -> &str {
    match blocker_code {
        "dispatch_transport_not_host_tool_bridge" => {
            taskflow_contracts::BlockerCode::HostBridgeRequestWrongTransport.as_str()
        }
        "receipt_mode_not_host_bridge_receipt" => {
            taskflow_contracts::BlockerCode::HostBridgeReceiptModeMismatch.as_str()
        }
        "request_status_not_admissible" => {
            taskflow_contracts::BlockerCode::HostBridgeRequestNotPending.as_str()
        }
        "run_id_mismatch" | "task_id_mismatch" | "dispatch_target_mismatch" => {
            taskflow_contracts::BlockerCode::HostBridgeRequestIdentityMismatch.as_str()
        }
        code => code,
    }
}

fn decision(blocker_codes: Vec<String>) -> HostBridgeProvenanceDecision {
    if blocker_codes.is_empty() {
        HostBridgeProvenanceDecision {
            accepted: true,
            blocker_codes,
            reason: "host bridge request provenance matches the declared dispatch identity"
                .to_string(),
        }
    } else {
        HostBridgeProvenanceDecision {
            accepted: false,
            blocker_codes,
            reason: "host bridge request provenance rejected fail-closed".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::minimal_request;

    #[test]
    fn provenance_rejects_transport_mismatch() {
        let mut request = minimal_request();
        request.dispatch_transport = "other".to_string();

        let decision = validate_host_bridge_request_provenance(&HostBridgeProvenanceInput {
            request,
            expected_run_id: Some("run-1".to_string()),
            expected_task_id: Some("task-1".to_string()),
            expected_dispatch_target: Some("developer".to_string()),
        });

        assert!(!decision.accepted);
        assert!(
            decision
                .blocker_codes
                .contains(&"dispatch_transport_not_host_tool_bridge".to_string())
        );
    }

    #[test]
    fn provenance_public_blocker_mapping_is_shared() {
        assert_eq!(
            host_bridge_provenance_public_blocker_code("request_status_not_admissible"),
            "host_bridge_request_not_pending"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("dispatch_target_mismatch"),
            "host_bridge_request_identity_mismatch"
        );
    }
}
