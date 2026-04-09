use serde_json::Value;

pub(crate) use crate::release1_contracts::{
    BlockerCode, CompatibilityBoundary, CompatibilityClass,
};

use crate::contract_profile_registry::{selected_contract_profile_id, ContractProfileId};
use crate::operator_contracts::RELEASE1_OPERATOR_CONTRACT_SPEC;

pub(crate) fn blocker_code(code: BlockerCode) -> Option<String> {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => crate::release1_contracts::blocker_code_value(code),
    }
}

pub(crate) fn blocker_code_str(code: BlockerCode) -> &'static str {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => crate::release1_contracts::blocker_code_str(code),
    }
}

pub(crate) fn canonical_blocker_codes(entries: &[String]) -> Vec<String> {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => crate::release1_contracts::canonical_blocker_code_list(
            entries.iter().map(String::as_str),
        ),
    }
}

pub(crate) fn canonical_blocker_code_list<I, S>(entries: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
            crate::release1_contracts::canonical_blocker_code_list(entries)
        }
    }
}

pub(crate) fn release_contract_status(ready: bool) -> &'static str {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
            crate::release1_contracts::release1_contract_status_str(ready)
        }
    }
}

pub(crate) fn boot_compatibility_is_backward_compatible(classification: &str) -> bool {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
            crate::release1_contracts::canonical_compatibility_class_str(classification)
                == Some(crate::release1_contracts::CompatibilityClass::BackwardCompatible.as_str())
        }
    }
}

pub(crate) fn canonical_compatibility_class_str(value: &str) -> Option<&'static str> {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
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
        ContractProfileId::Release1 => {
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
        ContractProfileId::Release1 => {
            crate::operator_contracts::render_operator_contract_envelope(
                &RELEASE1_OPERATOR_CONTRACT_SPEC,
                status,
                blocker_codes,
                next_actions,
                artifact_refs,
            )
        }
    }
}

pub(crate) fn operator_contracts_consistency_error(
    status: &str,
    blocker_codes: &[String],
    next_actions: &[String],
) -> Option<String> {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
            crate::operator_contracts::release1_operator_contracts_consistency_error(
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
        ContractProfileId::Release1 => {
            crate::operator_contracts::shared_operator_output_contract_parity_error(summary_json)
        }
    }
}

pub(crate) fn classify_compatibility_boundary(value: &str) -> CompatibilityBoundary {
    match selected_contract_profile_id() {
        ContractProfileId::Release1 => {
            crate::release1_contracts::classify_compatibility_boundary(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_contract_status_defaults_to_release1_vocabulary() {
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
}
