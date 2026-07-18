pub mod adapter_contract;
pub mod adapter_payload;
pub mod artifact_scope;
pub mod completion;
pub mod completion_authority;
pub mod errors;
pub mod legacy_normalization;
pub mod provenance;
pub mod receipt_binding;
pub mod request;

pub use adapter_contract::{HostBridgeAdapterContractError, HostBridgeAdapterOperations};
pub use adapter_payload::{
    build_host_bridge_adapter_payload, host_bridge_operator_fields, HostBridgeAdapterPayloadInput,
};
pub use artifact_scope::{
    attach_host_bridge_implementation_artifact,
    build_host_bridge_normalized_implementation_artifact, host_bridge_artifact_file,
    host_bridge_changed_files_from_artifact, host_bridge_normalized_implementation_artifact_path,
    host_bridge_record_component, host_bridge_request_implementation_artifacts,
    normalized_host_bridge_attempt_id, normalized_host_bridge_consolidation_receipt_id,
    push_unique_host_bridge_implementation_artifact, validate_implementation_artifact_scope,
    validate_implementation_artifact_scope_with_proof_paths,
    write_host_bridge_normalized_implementation_artifact, write_host_bridge_request,
    HostBridgeImplementationArtifact, HostBridgeNormalizedImplementationArtifact,
    ImplementationArtifactScopeDecision,
};
pub use completion::{
    host_bridge_artifact_has_retryable_completion_blocker,
    host_bridge_completed_artifact_status_is_admissible,
    host_bridge_completed_result_execution_state_is_admissible,
    host_bridge_completed_result_has_preview_refresh_evidence,
    host_bridge_completed_result_status_is_admissible,
    host_bridge_completion_authorized_request_artifacts, host_bridge_completion_identity_matches,
    host_bridge_completion_requires_implementation_artifacts,
    host_bridge_completion_retryable_blocker, host_bridge_completion_verdict,
    host_bridge_existing_request_status_is_admissible,
    host_bridge_request_allows_parent_adapter_dispatch,
    host_bridge_request_artifacts_are_bare_completion_candidates,
    host_bridge_request_effectively_requires_implementation_artifacts,
    host_bridge_request_has_implementation_artifact_contract,
    host_bridge_request_requires_implementation_artifacts,
    host_bridge_request_status_after_completion,
    host_bridge_request_status_allows_parent_completion,
    host_bridge_result_declares_no_code_change, host_bridge_result_verdict_contract_blockers,
    host_bridge_result_verdict_fields, host_bridge_result_verdict_fields_for_gate,
    host_bridge_result_verdict_fields_for_gate_and_next,
    materialize_host_bridge_completion_evidence, normalize_host_bridge_provenance_for_completion,
    HostBridgeCompletionEvidence, HostBridgeCompletionInput, HostBridgeCompletionVerdict,
    HostBridgeResultVerdictFields,
};
pub use completion_authority::{
    completion_authority_transition_matrix, decide_host_bridge_completion_authority,
    summary_blocker_codes, summary_text_reports_blocked_completion,
    HostBridgeCompletionAuthorityDecision, HostBridgeCompletionAuthorityInput,
    HostBridgeCompletionEffectIntent, HostBridgeCompletionEvent, HostBridgeCompletionFsm,
    HostBridgeCompletionState, HostBridgeCompletionTransition, HostBridgeCompletionTransitionCase,
    BLOCKER_OUTCOME_CONTRADICTION, BLOCKER_PROVENANCE_REJECTED, BLOCKER_RECEIPT_NOT_BOUND,
    BLOCKER_SUMMARY_DERIVED, BLOCKER_TYPED_BLOCKED_OUTCOME, BLOCKER_TYPED_FAILED_OUTCOME,
};
pub use errors::HostBridgeError;
pub use legacy_normalization::{
    normalize_legacy_command_options, normalize_legacy_host_bridge_completion_result,
    normalize_legacy_lane_completion, normalize_legacy_receipt, normalize_legacy_run_status,
    CompletionBlocker, CompletionOutcome, FlowStepRef, LegacyHostBridgeCompletionNormalization,
    LegacyHostBridgeCompletionNormalizationError, LEGACY_COMMAND_OPTIONS_SOURCE_CONTRACT_VERSION,
    LEGACY_HOST_BRIDGE_SOURCE_CONTRACT_VERSION, LEGACY_LANE_COMPLETION_SOURCE_CONTRACT_VERSION,
    LEGACY_OUTCOME_CONTRADICTION, LEGACY_RECEIPT_SOURCE_CONTRACT_VERSION,
    LEGACY_RUN_STATUS_SOURCE_CONTRACT_VERSION,
};
pub use provenance::{
    host_bridge_provenance_public_blocker_code, validate_host_bridge_request_provenance,
    HostBridgeProvenanceDecision, HostBridgeProvenanceInput,
};
pub use receipt_binding::{
    validate_dispatch_receipt_binding, DispatchReceiptBindingDecision, DispatchReceiptBindingInput,
};
pub use request::{
    default_host_bridge_required_result_fields, effective_host_bridge_request,
    effective_host_bridge_request_with_registry, host_bridge_blocked_result_contract,
    host_bridge_blocked_result_contract_allowed_next_node,
    host_bridge_blocked_result_contract_has_retry_evidence,
    host_bridge_blocked_result_contract_is_retryable, host_bridge_path_array,
    host_bridge_request_owned_paths, host_bridge_request_proof_artifact_paths,
    host_bridge_request_string, host_bridge_request_task_class, host_bridge_required_result_fields,
    legacy_internal_subagents_host_bridge_request, read_host_bridge_request, HostBridgeRequest,
    HostBridgeRequestPath, HOST_BRIDGE_REQUIRED_RESULT_FIELDS,
};

/// Compares host-bridge packet paths by runtime identity, not presentation spelling.
#[must_use]
pub fn host_bridge_packet_paths_equivalent(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    taskflow_core::runtime_packet_identity::runtime_packet_paths_equivalent(
        strip_windows_extended_prefix(left),
        strip_windows_extended_prefix(right),
    )
}

fn strip_windows_extended_prefix(path: &str) -> &str {
    let path = path.trim();
    path.strip_prefix(r"\\?\")
        .or_else(|| path.strip_prefix("//?/"))
        .unwrap_or(path)
}

/// Returns identity blockers for a submitted result without allowing a receipt
/// id to select a different request or packet.
#[must_use]
pub fn host_bridge_dispatch_identity_blockers(
    request: &serde_json::Value,
    result: &serde_json::Value,
) -> Vec<String> {
    let mut blockers = Vec::new();
    for (request_field, result_field) in [
        ("request_id", "request_id"),
        ("run_id", "run_id"),
        ("task_id", "task_id"),
        ("dispatch_target", "dispatch_target"),
        ("packet_id", "packet_id"),
        ("attempt_id", "attempt_id"),
        ("backend_id", "backend_id"),
    ] {
        let Some(expected) = request
            .get(request_field)
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(expected) = (!expected.trim().is_empty()).then_some(expected) else {
            continue;
        };
        let actual = result
            .get(result_field)
            .or_else(|| {
                if result_field == "backend_id" {
                    result.get("selected_backend")
                } else {
                    None
                }
            })
            .and_then(serde_json::Value::as_str);
        if actual != Some(expected) {
            blockers.push(format!(
                "host_bridge_result_identity_mismatch:{request_field}"
            ));
        }
    }
    for (request_field, result_field) in [("packet_path", "source_dispatch_packet_path")] {
        let Some(expected) = request
            .get(request_field)
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let matches = result
            .get(result_field)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|actual| host_bridge_packet_paths_equivalent(expected, actual));
        if !matches {
            blockers.push(format!(
                "host_bridge_result_identity_mismatch:{request_field}"
            ));
        }
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use crate::request::HostBridgeRequest;

    pub(crate) fn minimal_request() -> HostBridgeRequest {
        HostBridgeRequest {
            schema_version: 1,
            status: "pending".to_string(),
            request_id: "req-1".to_string(),
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            dispatch_target: "developer".to_string(),
            packet_path: PathBuf::from("runtime-consumption/packet.json"),
            backend_id: "internal_subagents".to_string(),
            carrier_id: "senior".to_string(),
            execution_boundary: "parent_host_session".to_string(),
            dispatch_transport: "host_tool_bridge".to_string(),
            receipt_mode: "host_bridge_receipt".to_string(),
            adapter_kind: "codex_host_tools".to_string(),
            adapter_capability_id: "codex.multi_agent_v1".to_string(),
            invocation_mode: "parent_host_tool_api".to_string(),
            adapter_contract_source: "request".to_string(),
            adapter_operations: Some(
                crate::HostBridgeAdapterOperations::from_registry_value(&json!({
                    "adapter_kind": "codex_host_tools",
                    "adapter_capability_id": "codex.multi_agent_v1",
                    "invocation_mode": "parent_host_tool_api",
                    "dispatch_transport": "host_tool_bridge",
                    "receipt_mode": "host_bridge_receipt",
                    "operations": {
                        "spawn": "multi_agent_v1.spawn_agent",
                        "wait": "multi_agent_v1.wait_agent",
                        "dispose": "multi_agent_v1.close_agent"
                    },
                    "dispose_policy": "configured"
                }))
                .expect("test adapter contract"),
            ),
            request_path: PathBuf::from("host-tool-bridge/requests/request.json"),
            result_path: PathBuf::from("host-tool-bridge/results/result.json"),
            receipt_path: PathBuf::from("host-tool-bridge/receipts/receipt.json"),
            required_result_fields: crate::request::default_host_bridge_required_result_fields(),
            owned_paths: vec![PathBuf::from("crates/taskflow-host-bridge")],
            raw: json!({}),
        }
    }

    #[test]
    fn dispatch_identity_rejects_stale_retry_result_and_accepts_current_packet() {
        let root = std::env::temp_dir().join(format!(
            "taskflow-host-bridge-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet directory");
        let current_packet = packet_dir.join("tester-2.json");
        let stale_packet = packet_dir.join("tester-1.json");
        std::fs::write(&current_packet, "{}").expect("write current packet");
        std::fs::write(&stale_packet, "{}").expect("write stale packet");
        let current_packet = current_packet.display().to_string();
        let stale_packet = stale_packet.display().to_string();
        let request = serde_json::json!({
            "request_id": "request-tester-2",
            "run_id": "run-retry",
            "task_id": "task-retry",
            "dispatch_target": "tester",
            "attempt_id": "attempt-tester-2",
            "packet_id": "packet-tester-2",
            "packet_path": current_packet.clone(),
            "backend_id": "internal_subagents"
        });
        let current_result = serde_json::json!({
            "request_id": "request-tester-2",
            "run_id": "run-retry",
            "task_id": "task-retry",
            "dispatch_target": "tester",
            "attempt_id": "attempt-tester-2",
            "packet_id": "packet-tester-2",
            "source_dispatch_packet_path": current_packet,
            "selected_backend": "internal_subagents"
        });
        assert!(
            super::host_bridge_dispatch_identity_blockers(&request, &current_result).is_empty()
        );

        let stale_result = serde_json::json!({
            "request_id": "request-tester-1",
            "run_id": "run-retry",
            "task_id": "task-retry",
            "dispatch_target": "tester",
            "attempt_id": "attempt-tester-1",
            "packet_id": "packet-tester-1",
            "source_dispatch_packet_path": stale_packet,
            "selected_backend": "internal_subagents"
        });
        assert!(
            super::host_bridge_dispatch_identity_blockers(&request, &stale_result)
                .iter()
                .any(|blocker| blocker.ends_with(":attempt_id"))
        );
        assert!(
            super::host_bridge_dispatch_identity_blockers(&request, &stale_result)
                .iter()
                .any(|blocker| blocker.ends_with(":packet_path"))
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn packet_path_identity_accepts_extended_and_mixed_spellings_but_rejects_other_packet() {
        let root = std::env::temp_dir().join(format!(
            "taskflow-host-bridge-path-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        std::fs::create_dir_all(&packet_dir).expect("create packet directory");
        let packet = packet_dir.join("current.json");
        let other = packet_dir.join("other.json");
        std::fs::write(&packet, "{}").expect("write packet");
        std::fs::write(&other, "{}").expect("write other packet");
        let normal = packet.display().to_string();
        let extended = format!(r"\\?\{}", normal);
        let mixed = normal.replace('\\', "/");

        assert!(super::host_bridge_packet_paths_equivalent(
            &normal, &extended
        ));
        assert!(super::host_bridge_packet_paths_equivalent(&normal, &mixed));
        assert!(!super::host_bridge_packet_paths_equivalent(
            &normal,
            &other.display().to_string()
        ));

        let _ = std::fs::remove_dir_all(root);
    }
}
