use serde_json::Value;

pub(crate) use crate::release1_contracts::{
    BlockerCode, CompatibilityBoundary, CompatibilityClass,
};

use crate::contract_profile_registry::{ContractProfileId, selected_contract_profile_id};
use crate::release1_operator_output::RELEASE1_OPERATOR_CONTRACT_SPEC;

pub(crate) fn blocker_code(code: BlockerCode) -> Option<String> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => crate::release1_contracts::blocker_code_value(code),
    }
}

pub(crate) fn blocker_code_str(code: BlockerCode) -> &'static str {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => crate::release1_contracts::blocker_code_str(code),
    }
}

pub(crate) fn canonical_blocker_codes(entries: &[String]) -> Vec<String> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_blocker_code_list(
                entries.iter().map(String::as_str),
            )
        }
    }
}

pub(crate) fn canonical_blocker_code_list<I, S>(entries: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_blocker_code_list(entries)
        }
    }
}

pub(crate) fn release_contract_status(ready: bool) -> &'static str {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::release1_contract_status_str(ready)
        }
    }
}

pub(crate) fn boot_compatibility_is_backward_compatible(classification: &str) -> bool {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_compatibility_class_str(classification)
                == Some(crate::release1_contracts::CompatibilityClass::BackwardCompatible.as_str())
        }
    }
}

pub(crate) fn canonical_compatibility_class_str(value: &str) -> Option<&'static str> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_compatibility_class_str(value)
        }
    }
}

pub(crate) fn evaluate_policy_gate_protocol_binding(
    policy_gate: &str,
    receipt_hint: Option<&str>,
    runtime_ready: bool,
) -> Option<String> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::evaluate_policy_gate_protocol_binding(
                policy_gate,
                receipt_hint,
                runtime_ready,
            )
            .and_then(blocker_code)
        }
    }
}

pub(crate) fn render_operator_contract_envelope(
    status: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
) -> Value {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_operator_output::render_operator_contract_envelope(
                &RELEASE1_OPERATOR_CONTRACT_SPEC,
                status,
                blocker_codes,
                next_actions,
                artifact_refs,
            )
        }
    }
}

pub(crate) fn operator_contract_status_is_blocked(value: &Value) -> bool {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_operator_output::operator_contract_status_is_blocked(
                &RELEASE1_OPERATOR_CONTRACT_SPEC,
                value,
            )
        }
    }
}

pub(crate) fn canonical_approval_status_str(value: &str) -> Option<&'static str> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_approval_status_str(value)
        }
    }
}

pub(crate) fn canonical_gate_level_str(value: &str) -> Option<&'static str> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::canonical_gate_level_str(value)
        }
    }
}

pub(crate) fn operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_operator_output::release1_operator_contracts_consistency_error(
                status,
                blocker_codes,
                next_actions,
            )
        }
    }
}

pub(crate) fn shared_operator_output_contract_parity_error(
    summary_json: &Value,
) -> Option<&'static str> {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_operator_output::shared_operator_output_contract_parity_error(
                summary_json,
            )
        }
    }
}

pub(crate) fn classify_compatibility_boundary(value: &str) -> CompatibilityBoundary {
    match selected_contract_profile_id() {
        ContractProfileId::OperatorContracts => {
            crate::release1_contracts::classify_compatibility_boundary(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_contract_status_defaults_to_operator_contract_vocabulary() {
        assert_eq!(release_contract_status(true), "pass");
        assert_eq!(release_contract_status(false), "blocked");
    }

    #[test]
    fn backward_compatibility_is_reported_through_generic_adapter() {
        assert!(boot_compatibility_is_backward_compatible("compatible"));
        assert_eq!(
            canonical_compatibility_class_str("compatible"),
            Some("backward_compatible")
        );
        assert_eq!(
            classify_compatibility_boundary("compatible"),
            CompatibilityBoundary::Compatible
        );
    }

    #[test]
    fn blocker_adapters_preserve_canonical_codes_and_deduplicate_entries() {
        assert_eq!(
            blocker_code(BlockerCode::MissingPacket),
            Some("missing_packet".to_string())
        );
        assert_eq!(
            blocker_code_str(BlockerCode::MissingPacket),
            "missing_packet"
        );
        assert_eq!(
            canonical_blocker_codes(&[
                "missing_packet".to_string(),
                " unknown ".to_string(),
                "missing_packet".to_string(),
            ]),
            vec!["missing_packet".to_string()]
        );
        assert_eq!(
            canonical_blocker_code_list(["open_delegated_cycle", "open_delegated_cycle"]),
            vec!["open_delegated_cycle".to_string()]
        );
    }

    #[test]
    fn policy_gate_adapter_maps_missing_receipts_and_runtime_readiness() {
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", None, false),
            Some("missing_protocol_binding_receipt".to_string())
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), false),
            Some("protocol_binding_not_runtime_ready".to_string())
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("retrieval_evidence", Some("pb-1"), true),
            None
        );
        assert_eq!(
            evaluate_policy_gate_protocol_binding("unknown_gate", Some("pb-1"), true),
            Some("unsupported_blocker_code".to_string())
        );
    }

    #[test]
    fn operator_envelope_adapter_preserves_mirrors_and_detects_parity_drift() {
        let payload = render_operator_contract_envelope(
            "blocked",
            vec!["missing_packet".to_string()],
            vec!["inspect packet".to_string()],
            serde_json::json!({"path": "packet.json"}),
        );

        assert_eq!(payload["status"], "blocked");
        assert!(operator_contract_status_is_blocked(&payload["status"]));

        let mirrors = serde_json::json!({
            "status": "blocked",
            "blocker_codes": ["missing_packet"],
            "next_actions": ["inspect packet"],
        });
        let mut mirrored = payload.clone();
        mirrored["shared_fields"] = mirrors.clone();
        mirrored["operator_contracts"] = mirrors;
        assert_eq!(
            shared_operator_output_contract_parity_error(&mirrored),
            None
        );

        let mut drifted = mirrored;
        drifted["status"] = serde_json::json!("pass");
        assert!(shared_operator_output_contract_parity_error(&drifted).is_some());
        assert!(!operator_contract_status_is_blocked(&drifted["status"]));
    }

    #[test]
    fn approval_and_gate_adapters_fail_closed_for_unknown_values() {
        assert_eq!(canonical_approval_status_str("approved"), Some("approved"));
        assert_eq!(canonical_approval_status_str("not-a-status"), None);
        assert_eq!(canonical_gate_level_str("block"), Some("block"));
        assert_eq!(canonical_gate_level_str("not-a-level"), None);
        assert!(
            operator_contracts_consistency_error(
                "blocked",
                &["missing_packet".into()],
                &["inspect".into()]
            )
            .is_none()
        );
        assert!(operator_contracts_consistency_error("unknown", &[], &[]).is_some());
    }
}
