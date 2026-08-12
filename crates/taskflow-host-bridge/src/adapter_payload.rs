use std::path::Path;

use operator_output::{
    command_text::human_command,
    operator_contracts::{
        OperatorContractSpec, canonical_pass_blocked_contract_status_str,
        finalize_operator_surface_verdict,
    },
};
use serde_json::Value;
use taskflow_contracts::{Release1ContractStatus, release1_contract_status_str};

use crate::adapter_contract::HostBridgeAdapterOperations;
use crate::completion::{
    host_bridge_request_effectively_requires_implementation_artifacts,
    host_bridge_request_status_allows_parent_completion,
};
use crate::request::{
    HostBridgeRequest, default_host_bridge_required_result_fields, effective_host_bridge_request,
    host_bridge_blocked_result_contract, host_bridge_request_error_fields,
    host_bridge_request_string,
};

pub struct HostBridgeAdapterPayloadInput<'a> {
    pub request_path: &'a Path,
    pub request: &'a Value,
    pub provenance_blockers: Vec<String>,
    pub retryable_completion_request: bool,
    pub completion_command: String,
    pub artifact_attach_command: Option<String>,
}

pub fn host_bridge_operator_fields(
    status: &str,
    blocker_codes: Vec<String>,
    shared_next_actions: Vec<String>,
    operator_next_actions: Vec<String>,
    artifact_refs: Value,
) -> (Value, Value) {
    let spec = OperatorContractSpec {
        contract_id: "host-agent-bridge-adapter-v1",
        schema_version: "1",
        pass_status: Release1ContractStatus::Pass.as_str(),
        blocked_status: Release1ContractStatus::Blocked.as_str(),
        canonicalize_status: canonical_pass_blocked_contract_status_str,
        status_error_label: "canonical pass/blocked",
    };
    let mut verdict = finalize_operator_surface_verdict(
        &spec,
        status,
        blocker_codes,
        operator_next_actions,
        artifact_refs,
    );
    verdict.shared_fields["next_actions"] = serde_json::json!(shared_next_actions);
    (verdict.shared_fields, verdict.operator_contracts)
}

pub fn build_host_bridge_adapter_payload(input: HostBridgeAdapterPayloadInput<'_>) -> Value {
    let effective_request = effective_host_bridge_request(input.request);
    let request = &effective_request;
    let typed_request = HostBridgeRequest::from_value(request.clone());
    let missing = typed_request
        .as_ref()
        .err()
        .map(host_bridge_request_error_fields)
        .unwrap_or_default();
    let run_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.run_id.as_str())
        .or_else(|| host_bridge_request_string(request, "run_id"));
    let task_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.task_id.as_str())
        .or_else(|| host_bridge_request_string(request, "task_id"));
    let attempt_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.attempt_id.as_str())
        .or_else(|| host_bridge_request_string(request, "attempt_id"));
    let packet_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.packet_id.as_str())
        .or_else(|| host_bridge_request_string(request, "packet_id"));
    let request_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.request_id.as_str())
        .or_else(|| host_bridge_request_string(request, "request_id"));
    let dispatch_target = typed_request
        .as_ref()
        .ok()
        .map(|request| request.dispatch_target.as_str())
        .or_else(|| host_bridge_request_string(request, "dispatch_target"));
    let packet_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.packet_path.display().to_string())
        .or_else(|| host_bridge_request_string(request, "packet_path").map(ToOwned::to_owned));
    let backend_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.backend_id.as_str())
        .or_else(|| host_bridge_request_string(request, "backend_id"));
    let carrier_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.carrier_id.as_str())
        .or_else(|| host_bridge_request_string(request, "carrier_id"));
    let adapter_kind = typed_request
        .as_ref()
        .ok()
        .map(|request| request.adapter_kind.as_str())
        .or_else(|| host_bridge_request_string(request, "adapter_kind"));
    let adapter_capability_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.adapter_capability_id.as_str())
        .or_else(|| host_bridge_request_string(request, "adapter_capability_id"));
    let result_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.result_path.display().to_string())
        .or_else(|| host_bridge_request_string(request, "result_path").map(ToOwned::to_owned));
    let receipt_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.receipt_path.display().to_string())
        .or_else(|| host_bridge_request_string(request, "receipt_path").map(ToOwned::to_owned));
    let request_status = typed_request
        .as_ref()
        .ok()
        .map(|request| request.status.as_str())
        .unwrap_or("unknown");
    let dispatch_transport = typed_request
        .as_ref()
        .ok()
        .map(|request| request.dispatch_transport.as_str())
        .or_else(|| host_bridge_request_string(request, "dispatch_transport"));
    let invocation_mode = typed_request
        .as_ref()
        .ok()
        .map(|request| request.invocation_mode.as_str())
        .unwrap_or("");
    let adapter_contract_source = typed_request
        .as_ref()
        .ok()
        .map(|request| request.adapter_contract_source.as_str())
        .unwrap_or("");
    let adapter_operations = typed_request
        .as_ref()
        .ok()
        .and_then(|request| request.adapter_operations.clone());
    let required_result_fields = typed_request
        .as_ref()
        .ok()
        .map(|request| request.required_result_fields.clone())
        .unwrap_or_else(default_host_bridge_required_result_fields);
    let blocked_result_contract = host_bridge_blocked_result_contract(input.request)
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| {
            serde_json::json!({
                "execution_state": "blocked",
                "decision": "rework_required",
                "verdict": "rework_required",
                "required_result_fields": required_result_fields.clone(),
                "rework_target_required_when_blocked": true,
                "allowed_next_node": Value::Null,
                "allowed_blocker_codes": [
                    taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable.as_str(),
                    taskflow_contracts::BlockerCode::HostToolCapabilityMissing.as_str(),
                    "host_agent_execution_failed"
                ]
            })
        });
    let mut blocker_codes = input.provenance_blockers;
    if !missing.is_empty() {
        blocker_codes.push(
            taskflow_contracts::BlockerCode::HostBridgeRequestMissingFields
                .as_str()
                .to_string(),
        );
    }
    let configured_dispatch_transport = adapter_operations
        .as_ref()
        .map(|operations| operations.dispatch_transport.as_str());
    if dispatch_transport != Some("host_tool_bridge")
        || configured_dispatch_transport
            .is_some_and(|configured| dispatch_transport != Some(configured))
    {
        blocker_codes.push(
            taskflow_contracts::BlockerCode::HostBridgeRequestWrongTransport
                .as_str()
                .to_string(),
        );
    }
    if !host_bridge_request_status_allows_parent_completion(
        request_status,
        input.retryable_completion_request,
    ) {
        blocker_codes.push(
            taskflow_contracts::BlockerCode::HostBridgeRequestNotPending
                .as_str()
                .to_string(),
        );
    }
    if adapter_operations.is_none() {
        blocker_codes.push(
            taskflow_contracts::BlockerCode::HostToolCapabilityMissing
                .as_str()
                .to_string(),
        );
    }
    let status = release1_contract_status_str(blocker_codes.is_empty());
    let receipt_id = match (run_id, dispatch_target) {
        (Some(run_id), Some(dispatch_target)) => {
            format!("{run_id}-{dispatch_target}-host-bridge-receipt")
        }
        _ => "host-bridge-receipt".to_string(),
    };
    let requires_implementation_artifacts = dispatch_target.is_some_and(|target| {
        host_bridge_request_effectively_requires_implementation_artifacts(request, target)
    });
    let implementation_artifacts_present = request
        .get("implementation_artifacts")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty());
    let artifact_attach_command = if requires_implementation_artifacts
        && !implementation_artifacts_present
        && run_id.is_some()
    {
        input.artifact_attach_command
    } else {
        None
    };
    let host_tool_calls = if status == Release1ContractStatus::Pass.as_str() {
        adapter_operations
            .as_ref()
            .map(|operations| {
                operations
                    .operation_sequence()
                    .into_iter()
                    .map(|tool| serde_json::json!({ "tool": tool }))
                    .collect::<Vec<_>>()
            })
            .map(Value::Array)
            .unwrap_or_else(|| Value::Array(Vec::new()))
    } else {
        serde_json::json!([])
    };
    let adapter_capacity_status = if status == Release1ContractStatus::Pass.as_str() {
        "ready_to_attempt"
    } else {
        "not_checked_due_request_blockers"
    };
    let adapter_capacity = serde_json::json!({
        "status": adapter_capacity_status,
        "capacity_observable": true,
        "capacity_source": "host_agent_handle_registry",
        "active_agents_count": 0,
        "thread_limit_reached": Value::Null,
        "registry_update_required": true,
        "blocked_result_code": taskflow_contracts::BlockerCode::HostAgentCapacityUnavailable.as_str(),
        "next_actions": [
            "Invoke the configured spawn operation from the parent host session when capacity is available.",
            "If the parent host tool reports thread or capacity exhaustion, submit a blocked host bridge result with blocker_code host_agent_capacity_unavailable so the host-agent handle registry records capacity state."
        ]
    });
    let durable_job_id = request_id
        .unwrap_or("")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let durable_job = serde_json::json!({
        "job_type": "vida.host_bridge.adapter_request",
        "job_id": format!("host-bridge-request-{durable_job_id}"),
        "idempotency_key": request_id,
        "request_id": request_id,
        "status": adapter_capacity_status,
        "authority": "host_bridge_request",
        "runner": "parent_host_adapter",
        "duplicate_enqueue_behavior": "resume_existing_request_id_job",
        "restart_replay_behavior": "resume_or_dead_letter_from_request_status",
        "dead_letter_blocker_code": "host_bridge_adapter_request_dead_letter"
    });
    let next_actions = if status == Release1ContractStatus::Pass.as_str() {
        artifact_attach_command
            .iter()
            .chain(std::iter::once(&input.completion_command))
            .map(|command| human_command(command))
            .collect::<Vec<_>>()
    } else {
        vec![
            "repair the host bridge request or selected host adapter capability before invoking parent host tools"
                .to_string(),
        ]
    };
    let artifact_refs = serde_json::json!({
        "request_path": input.request_path.display().to_string(),
        "packet_path": packet_path,
        "result_path": result_path,
        "receipt_path": receipt_path,
        "implementation_artifacts_present": implementation_artifacts_present
    });
    let (shared_fields, operator_contracts) = host_bridge_operator_fields(
        status,
        blocker_codes.clone(),
        next_actions.clone(),
        next_actions,
        artifact_refs,
    );
    serde_json::json!({
        "surface": "vida agent host-bridge",
        "status": status,
        "blocker_codes": blocker_codes,
        "shared_fields": shared_fields,
        "operator_contracts": operator_contracts,
        "host_bridge": {
            "request_path": input.request_path.display().to_string(),
            "request_status": request_status,
            "run_id": run_id,
            "task_id": task_id,
            "attempt_id": attempt_id,
            "packet_id": packet_id,
            "dispatch_target": dispatch_target,
            "packet_path": packet_path,
            "backend_id": backend_id,
            "carrier_id": carrier_id,
            "dispatch_transport": dispatch_transport,
            "adapter_kind": adapter_kind,
            "adapter_capability_id": adapter_capability_id,
            "invocation_mode": invocation_mode,
            "adapter_contract_source": adapter_contract_source,
            "adapter_operations": adapter_operations
                .as_ref()
                .map(HostBridgeAdapterOperations::to_value),
            "required_result_fields": required_result_fields.clone(),
            "missing_fields": missing,
            "result_path": result_path,
            "receipt_path": receipt_path,
            "receipt_id": receipt_id,
            "completion_command": input.completion_command,
            "artifact_attach_required": requires_implementation_artifacts && !implementation_artifacts_present,
            "artifact_attach_command": artifact_attach_command,
            "implementation_artifacts_present": implementation_artifacts_present,
            "host_tool_calls": host_tool_calls,
            "adapter_capacity": adapter_capacity,
            "durable_job": durable_job,
            "blocked_result_contract": blocked_result_contract,
            "binary_boundary": "vida.exe emits and validates bridge artifacts; parent host adapter invokes native host tools"
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::*;

    fn request() -> Value {
        let mut request = json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
            "task_id": "task-1",
            "attempt_id": "attempt-1",
            "packet_id": "packet-1",
            "backend_id": "internal_subagents",
            "carrier_id": "junior",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "receipt_mode": "host_bridge_receipt",
            "adapter_operations": {
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
            },
            "request_path": "request.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json"
        });
        let snapshot = request["adapter_operations"].clone();
        request["adapter_contract_snapshot"] = snapshot.clone();
        request["adapter_contract_hash"] = json!(
            blake3::hash(&serde_json::to_vec(&snapshot).expect("snapshot serializes"))
                .to_hex()
                .to_string()
        );
        request["adapter_contract_source"] = json!("configured_registry");
        request
    }

    fn payload_for(request: &Value) -> Value {
        let dispatch_target = request["dispatch_target"].as_str().unwrap_or("implementer");
        build_host_bridge_adapter_payload(HostBridgeAdapterPayloadInput {
            request_path: Path::new("request.json"),
            request,
            provenance_blockers: Vec::new(),
            retryable_completion_request: false,
            completion_command: format!("vida lane complete run-1 --receipt-id run-1-{dispatch_target}-host-bridge-receipt --host-bridge-request request.json --host-agent-id <host-agent-id> --host-bridge-result-file result.json"),
            artifact_attach_command: Some("vida agent host-bridge --request request.json --attach-artifact <artifact-path> --changed-file <changed-file> --artifact-kind patch_proposal".to_string()),
        })
    }

    #[test]
    fn host_bridge_adapter_payload_pass_renders_host_tool_contract() {
        let payload = payload_for(&request());

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["blocker_codes"].as_array().unwrap().len(), 0);
        assert_eq!(payload["shared_fields"]["status"], payload["status"]);
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            payload["operator_contracts"]["next_actions"]
        );
        assert_eq!(
            payload["operator_contracts"]["contract_id"],
            "host-agent-bridge-adapter-v1"
        );
        assert_eq!(
            payload["host_bridge"]["completion_command"],
            "vida lane complete run-1 --receipt-id run-1-implementer-host-bridge-receipt --host-bridge-request request.json --host-agent-id <host-agent-id> --host-bridge-result-file result.json"
        );
        assert_eq!(
            payload["host_bridge"]["artifact_attach_command"],
            "vida agent host-bridge --request request.json --attach-artifact <artifact-path> --changed-file <changed-file> --artifact-kind patch_proposal"
        );
        let calls = payload["host_bridge"]["host_tool_calls"]
            .as_array()
            .expect("host tool calls should render");
        assert_eq!(calls[0]["tool"], "multi_agent_v1.spawn_agent");
        assert_eq!(calls[1]["tool"], "multi_agent_v1.wait_agent");
        assert_eq!(calls[2]["tool"], "multi_agent_v1.close_agent");
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["status"],
            "ready_to_attempt"
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["capacity_observable"],
            true
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["capacity_source"],
            "host_agent_handle_registry"
        );
        assert_eq!(
            payload["host_bridge"]["durable_job"]["job_id"],
            "host-bridge-request-req-1"
        );
        assert_eq!(
            payload["host_bridge"]["durable_job"]["dead_letter_blocker_code"],
            "host_bridge_adapter_request_dead_letter"
        );
        assert_eq!(
            payload["host_bridge"]["required_result_fields"],
            json!([
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ])
        );
        assert_eq!(
            payload["host_bridge"]["blocked_result_contract"]["decision"],
            "rework_required"
        );
        assert_eq!(
            payload["host_bridge"]["blocked_result_contract"]["allowed_next_node"],
            Value::Null
        );
    }

    #[test]
    fn host_bridge_adapter_payload_echoes_explicit_blocked_result_contract_next_node() {
        let mut request = request();
        request["host_bridge"] = json!({
            "blocked_result_contract": {
                "decision": "rework_required",
                "verdict": "rework_required",
                "allowed_next_node": "alpha_rework"
            }
        });

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "pass");
        assert_eq!(
            payload["host_bridge"]["blocked_result_contract"]["allowed_next_node"],
            "alpha_rework"
        );
    }

    #[test]
    fn host_bridge_adapter_payload_advertises_attach_for_implementation_task_class() {
        let mut request = request();
        request["dispatch_target"] = json!("alpha_impl");
        request["task_class"] = json!("implementation");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["host_bridge"]["dispatch_target"], "alpha_impl");
        assert_eq!(payload["host_bridge"]["artifact_attach_required"], true);
        assert_eq!(
            payload["host_bridge"]["artifact_attach_command"],
            "vida agent host-bridge --request request.json --attach-artifact <artifact-path> --changed-file <changed-file> --artifact-kind patch_proposal"
        );
        assert!(
            payload["shared_fields"]["next_actions"]
                .as_array()
                .expect("next actions")
                .first()
                .and_then(serde_json::Value::as_str)
                .is_some_and(|action| action.contains("--attach-artifact"))
        );
    }

    #[test]
    fn host_bridge_adapter_payload_advertises_attach_for_implementation_dispatch_target() {
        let mut request = request();
        request["dispatch_target"] = json!("implementer");
        request["task_class"] = json!("quality_gate");
        request["implementation_artifacts"] = json!([]);

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["host_bridge"]["dispatch_target"], "implementer");
        assert_eq!(payload["host_bridge"]["artifact_attach_required"], true);
        assert_eq!(
            payload["host_bridge"]["artifact_attach_command"],
            "vida agent host-bridge --request request.json --attach-artifact <artifact-path> --changed-file <changed-file> --artifact-kind patch_proposal"
        );
        assert!(
            payload["shared_fields"]["next_actions"]
                .as_array()
                .expect("next actions")
                .first()
                .and_then(serde_json::Value::as_str)
                .is_some_and(|action| action.contains("--attach-artifact"))
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_missing_core_payload_fields() {
        let mut request = request();
        request.as_object_mut().unwrap().remove("packet_path");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_bridge_request_missing_fields")
        );
        assert_eq!(
            payload["host_bridge"]["missing_fields"],
            json!(["packet_path"])
        );
        assert_eq!(
            payload["host_bridge"]["adapter_capacity"]["status"],
            "not_checked_due_request_blockers"
        );
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            json!([
                "repair the host bridge request or selected host adapter capability before invoking parent host tools"
            ])
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_raw_unconfigured_host_agent_sentinels() {
        let mut request = request();
        request["adapter_kind"] = json!("unconfigured_host_agent_adapter");
        request["adapter_capability_id"] = json!("unconfigured_host_agent_capability");
        request["invocation_mode"] = json!("configured_host_capability_required");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_bridge_request_missing_fields")
        );
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_tool_capability_missing")
        );
        assert_eq!(
            payload["host_bridge"]["missing_fields"],
            json!(["adapter_operations"])
        );
        assert_eq!(payload["host_bridge"]["host_tool_calls"], json!([]));
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_missing_operation_with_operator_artifacts() {
        let mut request = request();
        request["adapter_operations"]["operations"]
            .as_object_mut()
            .unwrap()
            .remove("wait");
        let payload = payload_for(&request);
        assert_eq!(payload["status"], "blocked");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_tool_capability_missing")
        );
        assert!(
            payload["shared_fields"]["next_actions"]
                .as_array()
                .is_some_and(|actions| !actions.is_empty())
        );
        assert_eq!(
            payload["shared_fields"]["artifact_refs"]["request_path"],
            "request.json"
        );
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_wrong_transport() {
        let mut request = request();
        request["dispatch_transport"] = json!("filesystem");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert!(
            payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_bridge_request_wrong_transport")
        );
        assert_eq!(payload["host_bridge"]["dispatch_transport"], "filesystem");
        assert_eq!(payload["host_bridge"]["host_tool_calls"], json!([]));
    }

    #[test]
    fn malformed_typed_request_preserves_raw_identity_fallbacks() {
        let mut request = request();
        request
            .as_object_mut()
            .expect("request object")
            .remove("run_id");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["host_bridge"]["task_id"], "task-1");
        assert_eq!(payload["host_bridge"]["attempt_id"], "attempt-1");
        assert_eq!(payload["host_bridge"]["packet_id"], "packet-1");
        assert_eq!(payload["host_bridge"]["durable_job"]["request_id"], "req-1");
        assert_eq!(payload["host_bridge"]["dispatch_target"], "implementer");
        assert_eq!(payload["host_bridge"]["packet_path"], "packet.json");
        assert_eq!(payload["host_bridge"]["backend_id"], "internal_subagents");
        assert_eq!(payload["host_bridge"]["carrier_id"], "junior");
        assert_eq!(payload["host_bridge"]["adapter_kind"], "codex_host_tools");
        assert_eq!(
            payload["host_bridge"]["adapter_capability_id"],
            "codex.multi_agent_v1"
        );
        assert_eq!(
            payload["host_bridge"]["dispatch_transport"],
            "host_tool_bridge"
        );
        assert_eq!(payload["host_bridge"]["invocation_mode"], "");
        assert_eq!(payload["host_bridge"]["adapter_contract_source"], "");
        assert_eq!(payload["host_bridge"]["result_path"], "result.json");
        assert_eq!(payload["host_bridge"]["receipt_path"], "receipt.json");
    }

    #[test]
    fn blocked_operator_fields_preserve_status_and_blocker_contracts() {
        let (shared_fields, operator_contracts) = host_bridge_operator_fields(
            "blocked",
            vec!["host_bridge_request_wrong_transport".to_string()],
            vec!["repair the request".to_string()],
            vec!["repair the request".to_string()],
            json!({"request_path": "request.json"}),
        );

        assert_eq!(shared_fields["status"], "blocked");
        assert_eq!(
            shared_fields["blocker_codes"],
            json!(["host_bridge_request_wrong_transport"])
        );
        assert_eq!(shared_fields["next_actions"], json!(["repair the request"]));
        assert_eq!(operator_contracts["status"], "blocked");
        assert_eq!(
            operator_contracts["blocker_codes"],
            json!(["host_bridge_request_wrong_transport"])
        );
    }

    #[test]
    fn implementation_artifacts_suppress_attach_and_keep_completion_action() {
        let mut request = request();
        request["dispatch_target"] = json!("implementer");
        request["task_class"] = json!("implementation");
        request["implementation_artifacts"] = json!([{
            "artifact_path": "artifacts/patch.json",
            "changed_files": ["crates/taskflow-host-bridge/src/lib.rs"]
        }]);

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "pass");
        assert_eq!(
            payload["host_bridge"]["implementation_artifacts_present"],
            true
        );
        assert_eq!(payload["host_bridge"]["artifact_attach_required"], false);
        assert_eq!(
            payload["host_bridge"]["artifact_attach_command"],
            Value::Null
        );
        assert_eq!(
            payload["shared_fields"]["next_actions"],
            json!([
                "vida lane complete run-1 --receipt-id run-1-implementer-host-bridge-receipt --host-bridge-request request.json --host-agent-id <host-agent-id> --host-bridge-result-file result.json"
            ])
        );
    }

    #[test]
    fn valid_payload_preserves_identity_and_durable_job_contract() {
        let payload = payload_for(&request());
        let host_bridge = &payload["host_bridge"];

        for (field, expected) in [
            ("run_id", json!("run-1")),
            ("task_id", json!("task-1")),
            ("attempt_id", json!("attempt-1")),
            ("packet_id", json!("packet-1")),
            ("dispatch_target", json!("implementer")),
            ("packet_path", json!("packet.json")),
            ("backend_id", json!("internal_subagents")),
            ("carrier_id", json!("junior")),
            ("dispatch_transport", json!("host_tool_bridge")),
            ("adapter_kind", json!("codex_host_tools")),
            ("adapter_capability_id", json!("codex.multi_agent_v1")),
            ("invocation_mode", json!("parent_host_tool_api")),
            ("adapter_contract_source", json!("configured_registry")),
            ("result_path", json!("result.json")),
            ("receipt_path", json!("receipt.json")),
            ("receipt_id", json!("run-1-implementer-host-bridge-receipt")),
        ] {
            assert_eq!(host_bridge[field], expected, "identity field `{field}`");
        }
        assert_eq!(host_bridge["request_status"], "pending");
        assert_eq!(host_bridge["host_tool_calls"].as_array().unwrap().len(), 3);
        assert_eq!(host_bridge["durable_job"]["request_id"], "req-1");
        assert_eq!(host_bridge["durable_job"]["idempotency_key"], "req-1");
        assert_eq!(payload["operator_contracts"]["status"], "pass");
        assert_eq!(
            payload["shared_fields"]["artifact_refs"],
            json!({
                "request_path": "request.json",
                "packet_path": "packet.json",
                "result_path": "result.json",
                "receipt_path": "receipt.json",
                "implementation_artifacts_present": false
            })
        );
    }

    #[test]
    fn malformed_payload_uses_fail_closed_defaults_and_receipt_fallback() {
        let mut request = request();
        request.as_object_mut().unwrap().remove("run_id");

        let payload = payload_for(&request);
        let host_bridge = &payload["host_bridge"];

        assert_eq!(host_bridge["request_status"], "unknown");
        assert_eq!(host_bridge["receipt_id"], "host-bridge-receipt");
        assert_eq!(host_bridge["invocation_mode"], "");
        assert_eq!(host_bridge["adapter_contract_source"], "");
        assert_eq!(host_bridge["adapter_operations"], Value::Null);
        assert_eq!(
            host_bridge["required_result_fields"],
            json!([
                "decision",
                "verdict",
                "blocker_codes",
                "rework_target",
                "allowed_next_node"
            ])
        );
        assert_eq!(host_bridge["durable_job"]["request_id"], "req-1");
    }

    #[test]
    fn nested_transport_and_completed_status_emit_request_blockers() {
        let mut transport_mismatch = request();
        transport_mismatch["adapter_operations"]["dispatch_transport"] =
            json!("different_transport");
        transport_mismatch["dispatch_transport"] = json!("different_transport");
        let mismatch_payload = payload_for(&transport_mismatch);
        assert_eq!(mismatch_payload["status"], "blocked");
        assert!(
            mismatch_payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_bridge_request_wrong_transport")
        );
        assert_eq!(
            mismatch_payload["host_bridge"]["host_tool_calls"],
            json!([])
        );

        let mut completed = request();
        completed["status"] = json!("completed");
        let completed_payload = payload_for(&completed);
        assert_eq!(completed_payload["status"], "blocked");
        assert!(
            completed_payload["blocker_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|code| code == "host_bridge_request_not_pending")
        );
        assert_eq!(
            completed_payload["host_bridge"]["adapter_capacity"]["status"],
            "not_checked_due_request_blockers"
        );
    }
}
