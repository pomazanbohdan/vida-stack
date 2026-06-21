pub mod adapter_payload;
pub mod artifact_scope;
pub mod completion;
pub mod errors;
pub mod provenance;
pub mod receipt_binding;
pub mod request;

pub use adapter_payload::{
    HostBridgeAdapterPayloadInput, build_host_bridge_adapter_payload, host_bridge_operator_fields,
};
pub use artifact_scope::{
    HostBridgeImplementationArtifact, HostBridgeNormalizedImplementationArtifact,
    ImplementationArtifactScopeDecision, attach_host_bridge_implementation_artifact,
    build_host_bridge_normalized_implementation_artifact, host_bridge_artifact_file,
    host_bridge_changed_files_from_artifact, host_bridge_normalized_implementation_artifact_path,
    host_bridge_record_component, host_bridge_request_implementation_artifacts,
    normalized_host_bridge_attempt_id, normalized_host_bridge_consolidation_receipt_id,
    push_unique_host_bridge_implementation_artifact, validate_implementation_artifact_scope,
    write_host_bridge_normalized_implementation_artifact, write_host_bridge_request,
};
pub use completion::{
    HOST_AGENT_EXECUTION_RECEIPT_V2_SCHEMA_VERSION, HOST_AGENT_EXECUTION_RESULT_V2_SCHEMA_VERSION,
    HostAgentEvidenceContractDecision, HostAgentExecutionReceiptV2, HostAgentExecutionResultV2,
    HostBridgeCompletionEvidence, HostBridgeCompletionInput, HostBridgeCompletionVerdict,
    HostBridgeResultContractDecision, HostBridgeResultVerdictFields,
    host_agent_execution_evidence_v2_contract_decision,
    host_agent_execution_receipt_v2_contract_decision,
    host_agent_execution_result_v2_canonical_hash,
    host_agent_execution_result_v2_contract_decision,
    host_bridge_allowed_next_node_is_abstract_next,
    host_bridge_artifact_has_retryable_completion_blocker,
    host_bridge_completed_artifact_status_is_admissible,
    host_bridge_completed_result_execution_state_is_admissible,
    host_bridge_completed_result_status_is_admissible,
    host_bridge_completion_authorized_request_artifacts,
    host_bridge_completion_requires_implementation_artifacts,
    host_bridge_completion_retryable_blocker, host_bridge_completion_verdict,
    host_bridge_existing_request_status_is_admissible,
    host_bridge_lane_completion_summary_blocker_code,
    host_bridge_request_artifacts_are_bare_completion_candidates,
    host_bridge_request_requires_implementation_artifacts,
    host_bridge_request_status_after_completion,
    host_bridge_request_status_allows_parent_completion,
    host_bridge_result_contract_decision_for_target,
    host_bridge_result_contract_decision_for_target_with_authorized_next,
    host_bridge_result_decision_is_blocked, host_bridge_result_decision_is_pass,
    host_bridge_result_pass_allowed_next_node, host_bridge_result_verdict_contract_blockers,
    host_bridge_result_verdict_contract_blockers_for_target,
    host_bridge_result_verdict_contract_blockers_for_target_with_authorized_next,
    host_bridge_result_verdict_fields, host_bridge_result_verdict_fields_for_gate,
    host_bridge_result_verdict_fields_from_typed_result, host_bridge_result_verdict_is_blocked,
    host_bridge_result_verdict_is_pass, materialize_host_bridge_completion_evidence,
    normalize_host_bridge_provenance_for_completion,
};
pub use errors::HostBridgeError;
pub use provenance::{
    HostBridgeProvenanceDecision, HostBridgeProvenanceInput,
    host_bridge_provenance_public_blocker_code, validate_host_bridge_request_provenance,
};
pub use receipt_binding::{
    DispatchReceiptBindingDecision, DispatchReceiptBindingInput, validate_dispatch_receipt_binding,
};
pub use request::{
    HOST_BRIDGE_REQUIRED_RESULT_FIELDS, HostBridgeRequest, HostBridgeRequestPath,
    canonical_host_bridge_required_result_fields, default_host_bridge_required_result_fields,
    effective_host_bridge_request, host_bridge_path_array, host_bridge_request_owned_paths,
    host_bridge_request_string, host_bridge_required_result_fields,
    legacy_internal_subagents_host_bridge_request, read_host_bridge_request,
};

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
            dispatch_generation_id: Some("run-1::developer::generation-1".to_string()),
            lane_id: Some("developer".to_string()),
            dispatch_target: "developer".to_string(),
            runtime_role: Some("developer".to_string()),
            task_class: Some("implementation".to_string()),
            flow_id: None,
            flow_revision: None,
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
            request_path: PathBuf::from("host-tool-bridge/requests/request.json"),
            result_path: PathBuf::from("host-tool-bridge/results/result.json"),
            receipt_path: PathBuf::from("host-tool-bridge/receipts/receipt.json"),
            required_result_fields: crate::request::default_host_bridge_required_result_fields(),
            owned_paths: vec![PathBuf::from("crates/taskflow-host-bridge")],
            raw: json!({}),
        }
    }
}
