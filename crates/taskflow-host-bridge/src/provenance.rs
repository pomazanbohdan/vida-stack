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
    if !host_bridge_request_status_is_admissible_for_provenance(request) {
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

fn host_bridge_request_status_is_admissible_for_provenance(request: &HostBridgeRequest) -> bool {
    matches!(
        request.status.as_str(),
        "pending" | "pass" | "retryable_blocked"
    )
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
        "authoritative_state_store_locked" => {
            taskflow_contracts::BlockerCode::AuthoritativeStateStoreLocked.as_str()
        }
        "authoritative_state_store_open_failed" => {
            taskflow_contracts::BlockerCode::AuthoritativeStateStoreOpenFailed.as_str()
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
    fn provenance_rejects_receipt_mode_mismatch_with_fail_closed_reason() {
        let mut request = minimal_request();
        request.receipt_mode = "other".to_string();

        let decision = validate_host_bridge_request_provenance(&HostBridgeProvenanceInput {
            request,
            expected_run_id: Some("run-1".to_string()),
            expected_task_id: Some("task-1".to_string()),
            expected_dispatch_target: Some("developer".to_string()),
        });

        assert!(!decision.accepted);
        assert_eq!(
            decision.blocker_codes,
            vec!["receipt_mode_not_host_bridge_receipt"]
        );
        assert_eq!(
            decision.reason,
            "host bridge request provenance rejected fail-closed"
        );
    }

    #[test]
    fn provenance_public_blocker_mapping_is_shared() {
        assert_eq!(
            host_bridge_provenance_public_blocker_code("request_status_not_admissible"),
            "host_bridge_request_not_pending"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("dispatch_transport_not_host_tool_bridge"),
            "host_bridge_request_wrong_transport"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("receipt_mode_not_host_bridge_receipt"),
            "host_bridge_receipt_mode_mismatch"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("dispatch_target_mismatch"),
            "host_bridge_request_identity_mismatch"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("authoritative_state_store_locked"),
            "authoritative_state_store_locked"
        );
        assert_eq!(
            host_bridge_provenance_public_blocker_code("authoritative_state_store_open_failed"),
            "authoritative_state_store_open_failed"
        );
    }

    #[test]
    fn provenance_rejects_blocked_request_with_self_attested_retryable_result_contract() {
        let mut request = minimal_request();
        request.status = "blocked".to_string();
        request.raw["status"] = serde_json::json!("blocked");
        request.raw["blocked_result_contract"] = serde_json::json!({
            "status": "blocked",
            "decision": "rework_required",
            "verdict": "rework_required",
            "allowed_next_node": "gamma_rework",
            "rework_target": "gamma",
            "blocker_codes": ["proof_failed"]
        });

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
                .contains(&"request_status_not_admissible".to_string())
        );
    }

    #[test]
    fn provenance_accepts_each_admissible_request_status_with_matching_identity() {
        for status in ["pending", "pass", "retryable_blocked"] {
            let mut request = minimal_request();
            request.status = status.to_string();
            let decision = validate_host_bridge_request_provenance(&HostBridgeProvenanceInput {
                request,
                expected_run_id: Some("run-1".to_string()),
                expected_task_id: Some("task-1".to_string()),
                expected_dispatch_target: Some("developer".to_string()),
            });

            assert!(decision.accepted, "status `{status}` should be admissible");
            assert!(decision.blocker_codes.is_empty());
            assert_eq!(
                decision.reason,
                "host bridge request provenance matches the declared dispatch identity"
            );
        }
    }

    #[test]
    fn provenance_reports_each_expected_identity_mismatch() {
        for (field, blocker_code) in [
            ("run_id", "run_id_mismatch"),
            ("task_id", "task_id_mismatch"),
            ("dispatch_target", "dispatch_target_mismatch"),
        ] {
            let mut input = HostBridgeProvenanceInput {
                request: minimal_request(),
                expected_run_id: None,
                expected_task_id: None,
                expected_dispatch_target: None,
            };
            match field {
                "run_id" => input.expected_run_id = Some("other-run".to_string()),
                "task_id" => input.expected_task_id = Some("other-task".to_string()),
                "dispatch_target" => input.expected_dispatch_target = Some("tester".to_string()),
                _ => unreachable!(),
            }

            let decision = validate_host_bridge_request_provenance(&input);

            assert!(!decision.accepted);
            assert_eq!(decision.blocker_codes, vec![blocker_code.to_string()]);
            assert_eq!(
                decision.reason,
                "host bridge request provenance rejected fail-closed"
            );
        }
    }
}
