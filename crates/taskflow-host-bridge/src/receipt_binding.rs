use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
