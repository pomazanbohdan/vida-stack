use serde::{Deserialize, Serialize};
use serde_json::Value;
use taskflow_contracts::Release1ContractStatus;

use crate::request::HostBridgeRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchReceiptBindingInput {
    pub request: HostBridgeRequest,
    pub receipt: Option<Value>,
    pub allow_active_packet_target_override: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchReceiptBindingDecision {
    pub accepted: bool,
    pub blocker_codes: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeResultScaffoldInput {
    pub request: HostBridgeRequest,
    pub decision: Option<String>,
    pub verdict: Option<String>,
    pub blocker_codes: Vec<String>,
    pub rework_target: Option<String>,
    pub allowed_next_node: Option<String>,
    pub summary: Option<String>,
    pub host_agent_id: Option<String>,
    pub receipt_id: Option<String>,
}

pub fn validate_dispatch_receipt_binding(
    input: &DispatchReceiptBindingInput,
) -> DispatchReceiptBindingDecision {
    let Some(receipt) = input.receipt.as_ref() else {
        return rejected(vec!["missing_dispatch_receipt".to_string()]);
    };

    let mut blockers = Vec::new();
    if receipt.get("receipt_backed").is_some()
        && receipt.get("receipt_backed").and_then(Value::as_bool) != Some(true)
    {
        blockers.push("receipt_not_receipt_backed".to_string());
    }
    let active_dispatch_status = string_field(receipt, "dispatch_status").is_some_and(|status| {
        matches!(
            status,
            "routed" | "executing" | "bridge_request_pending" | "blocked"
        )
    });
    if string_field(receipt, "status").is_some_and(|status| status != "pass")
        && !active_dispatch_status
    {
        blockers.push("receipt_status_not_pass".to_string());
    }
    if string_field(receipt, "request_id").is_some()
        && string_field(receipt, "request_id") != Some(input.request.request_id.as_str())
    {
        blockers.push("receipt_request_id_mismatch".to_string());
    }
    if string_field(receipt, "run_id") != Some(input.request.run_id.as_str()) {
        blockers.push("receipt_run_id_mismatch".to_string());
    }
    if string_field(receipt, "dispatch_target") != Some(input.request.dispatch_target.as_str())
        && !input.allow_active_packet_target_override
    {
        blockers.push("receipt_dispatch_target_mismatch".to_string());
    }

    if blockers.is_empty() {
        DispatchReceiptBindingDecision {
            accepted: true,
            blocker_codes: blockers,
            reason: "dispatch receipt is bound to the host bridge request".to_string(),
        }
    } else {
        rejected(blockers)
    }
}

#[must_use]
pub fn build_host_bridge_result_scaffold(input: HostBridgeResultScaffoldInput) -> Value {
    let allowed_next_node = input.allowed_next_node.or_else(|| {
        input
            .request
            .raw
            .get("allowed_next_node")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    });
    let blocked = !input.blocker_codes.is_empty();
    let decision = input.decision.unwrap_or_else(|| {
        if blocked {
            "rework_required".to_string()
        } else {
            "approve".to_string()
        }
    });
    let verdict = input.verdict.unwrap_or_else(|| {
        if blocked {
            "blocked".to_string()
        } else {
            "pass".to_string()
        }
    });
    let status = if blocked {
        Release1ContractStatus::Blocked.as_str()
    } else {
        Release1ContractStatus::Pass.as_str()
    };
    let execution_state = if blocked { "blocked" } else { "executed" };
    let summary = input.summary.unwrap_or_else(|| {
        format!(
            "parent host adapter staged {verdict} result for {}",
            input.request.dispatch_target
        )
    });

    serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "host_tool_bridge_result",
        "status": status,
        "execution_state": execution_state,
        "request_id": input.request.request_id,
        "run_id": input.request.run_id,
        "task_id": input.request.task_id,
        "dispatch_target": input.request.dispatch_target,
        "decision": decision,
        "verdict": verdict,
        "blocker_codes": input.blocker_codes,
        "rework_target": input.rework_target,
        "allowed_next_node": allowed_next_node,
        "summary": summary,
        "execution_evidence": {
            "receipt_backed": true,
            "source": "vida_agent_host_bridge_scaffold",
            "host_agent_id": input.host_agent_id,
            "receipt_id": input.receipt_id
        },
        "source_dispatch_packet_path": input.request.packet_path,
        "identity_binding": {
            "request_id": input.request.request_id,
            "run_id": input.request.run_id,
            "task_id": input.request.task_id,
            "dispatch_target": input.request.dispatch_target
        }
    })
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn rejected(blocker_codes: Vec<String>) -> DispatchReceiptBindingDecision {
    DispatchReceiptBindingDecision {
        accepted: false,
        blocker_codes,
        reason: "dispatch receipt binding rejected fail-closed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::minimal_request;

    #[test]
    fn receipt_binding_rejects_missing_receipt() {
        let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
            request: minimal_request(),
            receipt: None,
            allow_active_packet_target_override: false,
        });

        assert!(!decision.accepted);
        assert_eq!(decision.blocker_codes, vec!["missing_dispatch_receipt"]);
    }

    #[test]
    fn receipt_binding_allows_active_packet_target_override() {
        let decision = validate_dispatch_receipt_binding(&DispatchReceiptBindingInput {
            request: minimal_request(),
            receipt: Some(serde_json::json!({
                "dispatch_status": "bridge_request_pending",
                "run_id": "run-1",
                "dispatch_target": "coach"
            })),
            allow_active_packet_target_override: true,
        });

        assert!(decision.accepted);
    }

    #[test]
    fn result_scaffold_defaults_required_fields_and_binds_identity() {
        let mut request = minimal_request();
        request.dispatch_target = "analyst".to_string();
        request.raw = serde_json::json!({
            "allowed_next_node": "pass_to_designer"
        });

        let result = build_host_bridge_result_scaffold(HostBridgeResultScaffoldInput {
            request,
            decision: None,
            verdict: None,
            blocker_codes: Vec::new(),
            rework_target: None,
            allowed_next_node: None,
            summary: None,
            host_agent_id: Some("host-agent-1".to_string()),
            receipt_id: Some("receipt-1".to_string()),
        });

        assert_eq!(result["artifact_kind"], "host_tool_bridge_result");
        assert_eq!(result["status"], "pass");
        assert_eq!(result["execution_state"], "executed");
        assert_eq!(result["decision"], "approve");
        assert_eq!(result["verdict"], "pass");
        assert_eq!(result["blocker_codes"], serde_json::json!([]));
        assert_eq!(
            result["source_dispatch_packet_path"],
            "runtime-consumption/packet.json"
        );
        assert!(result.get("rework_target").is_some());
        assert_eq!(result["allowed_next_node"], "pass_to_designer");
        assert_eq!(result["identity_binding"]["request_id"], "req-1");
        assert_eq!(result["identity_binding"]["run_id"], "run-1");
        assert_eq!(result["identity_binding"]["dispatch_target"], "analyst");
        assert_eq!(result["execution_evidence"]["receipt_backed"], true);
    }
}
