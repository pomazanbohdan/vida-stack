use std::path::Path;

use operator_output::{
    command_text::human_command,
    operator_contracts::{
        canonical_pass_blocked_contract_status_str, finalize_operator_surface_verdict,
        OperatorContractSpec,
    },
};
use serde_json::Value;
use taskflow_contracts::{release1_contract_status_str, Release1ContractStatus};

use crate::adapter_contract::HostBridgeAdapterOperations;
use crate::completion::{
    host_bridge_request_effectively_requires_implementation_artifacts,
    host_bridge_request_status_allows_parent_completion,
};
use crate::request::{
    default_host_bridge_required_result_fields, effective_host_bridge_request,
    host_bridge_blocked_result_contract, host_bridge_request_string, HostBridgeRequest,
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

fn push_missing_field(missing: &mut Vec<String>, field: &str) {
    if !missing.iter().any(|value| value == field) {
        missing.push(field.to_string());
    }
}

fn raw_host_bridge_request_missing_fields(request: &Value) -> Vec<String> {
    let mut missing = Vec::new();
    if HostBridgeAdapterOperations::from_request_value(request).is_err() {
        push_missing_field(&mut missing, "adapter_operations");
    }
    missing
}

pub fn build_host_bridge_adapter_payload(input: HostBridgeAdapterPayloadInput<'_>) -> Value {
    let raw_missing = raw_host_bridge_request_missing_fields(input.request);
    let effective_request = effective_host_bridge_request(input.request);
    let request = &effective_request;
    let mut missing = raw_missing;
    let typed_request = HostBridgeRequest::from_value(request.clone());
    if typed_request.is_err() {
        for field in [
            "run_id",
            "dispatch_target",
            "packet_path",
            "backend_id",
            "carrier_id",
            "adapter_kind",
            "adapter_capability_id",
            "result_path",
            "receipt_path",
        ] {
            if host_bridge_request_string(request, field).is_none() {
                missing.push(field.to_string());
            }
        }
    } else if let Ok(request) = typed_request.as_ref() {
        for (field, missing_path) in [
            ("packet_path", request.packet_path.as_os_str().is_empty()),
            ("result_path", request.result_path.as_os_str().is_empty()),
            ("receipt_path", request.receipt_path.as_os_str().is_empty()),
        ] {
            if missing_path {
                missing.push(field.to_string());
            }
        }
    }
    let run_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.run_id.as_str());
    let request_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.request_id.as_str())
        .or_else(|| host_bridge_request_string(request, "request_id"))
        .unwrap_or("unknown-request");
    let dispatch_target = typed_request
        .as_ref()
        .ok()
        .map(|request| request.dispatch_target.as_str());
    let packet_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.packet_path.display().to_string());
    let backend_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.backend_id.as_str());
    let carrier_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.carrier_id.as_str());
    let adapter_kind = typed_request
        .as_ref()
        .ok()
        .map(|request| request.adapter_kind.as_str());
    let adapter_capability_id = typed_request
        .as_ref()
        .ok()
        .map(|request| request.adapter_capability_id.as_str());
    let result_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.result_path.display().to_string());
    let receipt_path = typed_request
        .as_ref()
        .ok()
        .map(|request| request.receipt_path.display().to_string());
    let request_status = typed_request
        .as_ref()
        .ok()
        .map(|request| request.status.as_str())
        .unwrap_or("unknown");
    let dispatch_transport = typed_request
        .as_ref()
        .ok()
        .map(|request| request.dispatch_transport.as_str());
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
    let adapter_operations = HostBridgeAdapterOperations::from_request_value(request).ok();
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
    let configured_dispatch_transport = request
        .get("adapter_operations")
        .and_then(|value| HostBridgeAdapterOperations::from_registry_value(value).ok())
        .map(|operations| operations.dispatch_transport);
    if dispatch_transport.is_none_or(|value| value.trim().is_empty())
        || configured_dispatch_transport
            .as_deref()
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
        json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "dispatch_target": "implementer",
            "packet_path": "packet.json",
            "runtime_role": "worker",
            "task_class": "implementation",
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
        })
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
        assert!(payload["shared_fields"]["next_actions"]
            .as_array()
            .expect("next actions")
            .first()
            .and_then(serde_json::Value::as_str)
            .is_some_and(|action| action.contains("--attach-artifact")));
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
        assert!(payload["shared_fields"]["next_actions"]
            .as_array()
            .expect("next actions")
            .first()
            .and_then(serde_json::Value::as_str)
            .is_some_and(|action| action.contains("--attach-artifact")));
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_missing_core_payload_fields() {
        let mut request = request();
        request.as_object_mut().unwrap().remove("packet_path");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "host_bridge_request_missing_fields"));
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
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "host_bridge_request_missing_fields"));
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "host_tool_capability_missing"));
        assert_eq!(
            payload["host_bridge"]["missing_fields"],
            json!(["adapter_operations"])
        );
        assert_eq!(payload["host_bridge"]["host_tool_calls"], json!([]));
    }

    #[test]
    fn host_bridge_adapter_payload_blocks_wrong_transport() {
        let mut request = request();
        request["dispatch_transport"] = json!("filesystem");

        let payload = payload_for(&request);

        assert_eq!(payload["status"], "blocked");
        assert!(payload["blocker_codes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|code| code == "host_bridge_request_wrong_transport"));
        assert_eq!(payload["host_bridge"]["dispatch_transport"], "filesystem");
        assert_eq!(payload["host_bridge"]["host_tool_calls"], json!([]));
    }
}
