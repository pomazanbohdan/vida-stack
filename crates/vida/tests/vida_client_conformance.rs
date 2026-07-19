#[path = "../src/command_pipeline.rs"]
mod command_pipeline;
#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::task::Poll;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use taskflow_contracts::{
    VidaAggregateRef, VidaCommandRef, VidaDomainEventEnvelope, VidaEffectIntent, VidaEffectRef,
    VidaEventRef, VidaSchemaId, VidaSchemaVersion, VidaStreamRef, VidaStreamVersion, VidaTimestamp,
};
use taskflow_host_bridge::HostBridgeAdapterOperations;
use taskflow_state::{JournalAppendRequest, OperationalJournal};
use taskflow_state_redb::RedbOperationalJournal;
use vida_client::VidaClient;
use vida_client_inprocess::{InProcessVidaClient, LocalRuntimeVidaClient};
use vida_contracts::{
    operation_spec, operations, VidaApplyToken, VidaClientKind, VidaCommandEnvelope,
    VidaIdempotencyKey, VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId,
    VidaResponseStatus, VidaSessionId, VIDA_COMMAND_PROTOCOL_VERSION,
    VIDA_CONTRACTS_SCHEMA_VERSION,
};
use vida_runtime_local::jobs::{
    plan_host_bridge_request_job, DurableJobLifecycle, HostBridgeRequestJobSnapshot, RetryPolicy,
};

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("test-session".to_string()),
        request_id: VidaRequestId(format!("request-{operation}")),
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: VidaClientKind::Cli,
        project_ref: None,
        claim_kind: operation_spec(operation).map(|spec| spec.required_claim),
        trusted_owned_path: None,
        trusted_owned_write_scopes: Vec::new(),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("idem-{operation}"))),
        apply_token: None,
    }
}

fn execute(operation: &str) -> vida_contracts::VidaCommandResponse {
    let mut command = envelope(operation);
    command.project_ref = Some(local_project_ref());
    InProcessVidaClient::new_ready().execute(command)
}

fn local_project_ref() -> VidaProjectRef {
    let cwd = std::env::current_dir().expect("current dir");
    let root = cwd
        .ancestors()
        .find(|path| path.join("AGENTS.sidecar.md").is_file() || path.join(".vida").is_dir())
        .unwrap_or(&cwd);
    let project_id = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("vida-stack")
        .to_string();
    VidaProjectRef::ProjectId {
        project_id: VidaProjectId(project_id),
    }
}

fn current_host_bridge_request_fixture(
    state_root: &Path,
    request_path: &Path,
    attempt_count: u64,
) -> serde_json::Value {
    let project_root = std::env::current_dir()
        .expect("current dir")
        .ancestors()
        .find(|path| path.join("vida.config.yaml").is_file())
        .map(Path::to_path_buf)
        .expect("project config root");
    let config: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(project_root.join("vida.config.yaml")).expect("project config"),
    )
    .expect("project config yaml");
    let host_environment = config
        .get("host_environment")
        .expect("host environment config");
    let system_id = host_environment
        .get("cli_system")
        .and_then(serde_yaml::Value::as_str)
        .expect("configured host system");
    let system_id = system_id.trim();
    assert!(
        !system_id.is_empty(),
        "configured host system must be nonempty"
    );
    let system = host_environment
        .get("systems")
        .and_then(|systems| systems.get(system_id))
        .expect("configured host system entry");
    let adapter_value = system
        .get("host_tool_bridge")
        .expect("configured host bridge adapter");
    let adapter = HostBridgeAdapterOperations::from_registry_value(
        &serde_json::to_value(adapter_value).expect("adapter config json"),
    )
    .expect("configured host bridge adapter contract");
    let backend_id = config
        .get("party_chat")
        .and_then(|party_chat| party_chat.get("single_agent"))
        .and_then(|single_agent| single_agent.get("backend"))
        .and_then(serde_yaml::Value::as_str)
        .expect("configured backend")
        .trim()
        .to_string();
    assert!(
        !backend_id.is_empty(),
        "configured backend must be nonempty"
    );
    let worker_strategy: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(project_root.join(".vida/state/worker-strategy.json"))
            .expect("worker strategy"),
    )
    .expect("worker strategy json");
    let selected_tier = worker_strategy
        .get("agents")
        .and_then(|agents| agents.get(backend_id.as_str()))
        .and_then(|agent| agent.get("tier"))
        .and_then(serde_json::Value::as_str)
        .filter(|tier| !tier.trim().is_empty())
        .expect("backend carrier tier")
        .trim();
    assert!(
        !selected_tier.is_empty(),
        "backend carrier tier must be nonempty"
    );
    let carriers = system.get("carriers").expect("configured carrier catalog");
    let carrier_rows = carriers
        .as_mapping()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(id, carrier)| {
                    let map_key = id.as_str()?.trim();
                    if map_key.is_empty() {
                        return None;
                    }
                    let mut row = serde_json::to_value(carrier).ok()?;
                    let row_role_id = row
                        .get("role_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .unwrap_or(map_key);
                    assert!(
                        !row_role_id.is_empty(),
                        "carrier row role_id must be nonempty"
                    );
                    assert_eq!(
                        row_role_id, map_key,
                        "materialized carrier row role_id must match catalog map key"
                    );
                    row["role_id"] = serde_json::json!(row_role_id);
                    Some((map_key.to_string(), row))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let carrier_role_ids = carrier_rows
        .iter()
        .map(|(role_id, _)| role_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        carrier_role_ids.len(),
        carrier_rows.len(),
        "materialized carrier role ids must be globally unique"
    );
    let tier_carriers = carrier_rows
        .iter()
        .filter(|(_, carrier)| {
            carrier
                .get("tier")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                == Some(selected_tier)
        })
        .collect::<Vec<_>>();
    assert!(
        !tier_carriers.is_empty(),
        "backend tier must resolve host carriers"
    );
    let dev_team_roles = config
        .get("dev_team")
        .and_then(|dev_team| dev_team.get("roles"))
        .and_then(serde_yaml::Value::as_mapping)
        .expect("configured dev team roles");
    let mut route_candidates = Vec::<(String, String, String, String, String)>::new();
    for (carrier_id, carrier) in &tier_carriers {
        let runtime_roles = carrier
            .get("runtime_roles")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .collect::<Vec<_>>();
        let task_classes = carrier
            .get("task_classes")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|class| !class.is_empty())
            .collect::<Vec<_>>();
        for runtime_role in &runtime_roles {
            for task_class in &task_classes {
                for (role_id, role) in dev_team_roles {
                    let role_id = role_id.as_str().map(str::trim).unwrap_or_default();
                    let role_runtime = role
                        .get("runtime_role")
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    let role_task_match = role
                        .get("task_classes")
                        .and_then(serde_yaml::Value::as_sequence)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .any(|class| class == *task_class);
                    if role_id.is_empty() || role_runtime != *runtime_role || !role_task_match {
                        continue;
                    }
                    let dispatch_target = role
                        .get("dispatch_target")
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .filter(|target| !target.is_empty())
                        .unwrap_or(task_class);
                    route_candidates.push((
                        carrier_id.clone(),
                        runtime_role.to_string(),
                        task_class.to_string(),
                        role_id.to_string(),
                        dispatch_target.to_string(),
                    ));
                }
            }
        }
    }
    assert!(
        !route_candidates.is_empty(),
        "config must yield a host bridge route"
    );
    route_candidates.sort();
    let (carrier_id, runtime_role, task_class, role_id, dispatch_target) = route_candidates
        .into_iter()
        .next()
        .expect("deterministic host bridge route");
    assert!(
        !carrier_id.is_empty(),
        "configured carrier id must be nonempty"
    );
    assert!(
        !runtime_role.is_empty(),
        "configured runtime role must be nonempty"
    );
    assert!(
        !task_class.is_empty(),
        "configured task class must be nonempty"
    );
    assert!(!role_id.is_empty(), "configured role id must be nonempty");
    assert!(
        !dispatch_target.is_empty(),
        "configured dispatch target must be nonempty"
    );
    let carrier = tier_carriers
        .into_iter()
        .find(|(candidate_id, _)| candidate_id == &carrier_id)
        .map(|(_, carrier)| carrier.clone())
        .expect("selected configured carrier");
    assert!(
        carrier
            .get("runtime_roles")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .any(|role| role == runtime_role),
        "selected carrier must contain resolved runtime role"
    );
    assert!(
        carrier
            .get("task_classes")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .any(|class| class == task_class),
        "selected carrier must contain resolved task class"
    );
    let request_id = "req-scoped";
    let run_id = "run-scoped";
    let task_id = format!("{run_id}-task");
    let attempt_id = format!("{request_id}-attempt");
    let packet_id = format!("{request_id}-packet");
    let result_path = state_root.join("host-tool-bridge/results/result-1.json");
    let receipt_path = state_root.join("host-tool-bridge/receipts/receipt-1.json");
    let packet_path = state_root.join("host-tool-bridge/packets/packet-1.json");
    for (label, path) in [
        ("request", request_path),
        ("result", result_path.as_path()),
        ("receipt", receipt_path.as_path()),
        ("packet", packet_path.as_path()),
    ] {
        assert_state_scoped_path(state_root, path, label);
    }
    let adapter_contract_snapshot = adapter.to_value();
    let adapter_contract_hash = blake3::hash(
        &serde_json::to_vec(&adapter_contract_snapshot).expect("adapter snapshot json"),
    )
    .to_hex()
    .to_string();

    serde_json::json!({
        "schema_version": 1,
        "status": "failed",
        "request_id": request_id,
        "run_id": run_id,
        "task_id": task_id,
        "attempt_id": attempt_id,
        "packet_id": packet_id,
        "dispatch_target": dispatch_target,
        "attempt_count": attempt_count,
        "packet_path": packet_path.display().to_string(),
        "backend_id": backend_id,
        "carrier_id": carrier_id,
        "runtime_role": runtime_role,
        "task_class": task_class,
        "execution_boundary": system
            .get("execution_boundary")
            .and_then(serde_yaml::Value::as_str)
            .expect("configured execution boundary"),
        "dispatch_transport": adapter.dispatch_transport.clone(),
        "receipt_mode": adapter.receipt_mode.clone(),
        "adapter_kind": adapter.adapter_kind.clone(),
        "adapter_capability_id": adapter.adapter_capability_id.clone(),
        "invocation_mode": adapter.invocation_mode.clone(),
        "adapter_contract_source": project_root.join("vida.config.yaml").display().to_string(),
        "adapter_contract_snapshot": adapter_contract_snapshot,
        "adapter_contract_hash": adapter_contract_hash,
        "adapter_operations": adapter.to_value(),
        "request_path": request_path.display().to_string(),
        "result_path": result_path.display().to_string(),
        "receipt_path": receipt_path.display().to_string(),
        "required_result_fields": taskflow_host_bridge::default_host_bridge_required_result_fields(),
        "owned_paths": [],
        "failure_reason": "adapter exhausted inside state root"
    })
}

fn assert_state_scoped_path(state_root: &Path, path: &Path, label: &str) {
    let canonical_state_root = std::fs::canonicalize(state_root).expect("canonical state root");
    assert!(
        state_root.is_absolute(),
        "state root must be an absolute canonicalization anchor"
    );
    assert!(path.is_absolute(), "{label} path must be absolute");
    let relative = path
        .strip_prefix(state_root)
        .unwrap_or_else(|_| panic!("{label} path must remain below state root"));
    assert!(
        !relative.as_os_str().is_empty(),
        "{label} path must name a state file"
    );
    assert!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "{label} path must use component-safe descendants"
    );
    let canonical_candidate = canonical_state_root.join(relative);
    assert!(
        canonical_candidate.starts_with(&canonical_state_root),
        "{label} path must remain below canonical state root"
    );
}

#[test]
fn canonical_command_pipeline_layer_order_is_fixed() {
    use command_pipeline::CommandPipelineLayer::*;

    assert_eq!(
        command_pipeline::VidaCommandPipeline::<LocalRuntimeVidaClient>::layer_order(),
        &[
            Trace,
            Deadline,
            SchemaProtocol,
            OperationLookup,
            ProjectRouting,
            AuthorizationAdmission,
            Idempotency,
            Concurrency,
            Handler,
            ResponseMapping,
        ]
    );
}

#[test]
fn canonical_command_pipeline_trace_names_are_stable() {
    assert_eq!(
        command_pipeline::VidaCommandPipeline::<LocalRuntimeVidaClient>::trace_names(),
        vec![
            "trace",
            "deadline",
            "schema_protocol",
            "operation_lookup",
            "project_routing",
            "authorization_admission",
            "idempotency",
            "concurrency",
            "handler",
            "response_mapping"
        ]
    );
}

#[tokio::test]
async fn command_pipeline_tower_service_runs_under_tower_test_spawn() {
    let pipeline = command_pipeline::VidaCommandPipeline::new(LocalRuntimeVidaClient::new_ready());
    let mut service = tower_test::mock::Spawn::new(pipeline);

    assert!(matches!(
        service.poll_ready::<VidaCommandEnvelope>(),
        Poll::Ready(Ok(()))
    ));

    let mut command = envelope(operations::SERVICE_STATUS);
    command.project_ref = Some(local_project_ref());
    let response = service.call(command).await.expect("pipeline is infallible");

    assert_eq!(response.status, VidaResponseStatus::Pass);
    assert_eq!(response.result.expect("status result")["service"], "vida");
}

#[test]
fn command_pipeline_blocks_schema_protocol_mismatch_before_handler() {
    let mut command = envelope(operations::SERVICE_STATUS);
    command.schema_version = "wrong-schema".to_string();
    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(response.blockers[0].code, "schema_version_mismatch");
    assert!(response.result.is_none());
}

#[test]
fn command_pipeline_blocks_apply_operation_without_idempotency_and_apply_token() {
    let mut command = envelope(operations::SERVICE_LIFECYCLE_APPLY);
    command.client_kind = VidaClientKind::Service;
    command.idempotency_key = None;
    command.apply_token = None;
    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.blockers[0].code,
        "operation_idempotency_key_required"
    );
}

#[test]
fn command_pipeline_blocks_apply_operation_without_apply_token_after_idempotency() {
    let mut command = envelope(operations::SERVICE_LIFECYCLE_APPLY);
    command.client_kind = VidaClientKind::Service;
    command.apply_token = None;
    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(response.blockers[0].code, "operation_apply_token_required");
}

#[test]
fn command_pipeline_denies_payload_owned_scope_even_when_apply_token_is_present() {
    let mut command = envelope(operations::SERVICE_LIFECYCLE_APPLY);
    command.client_kind = VidaClientKind::Service;
    command.apply_token = Some(VidaApplyToken("test-apply-token".to_string()));
    command.payload = json!({
        "owned_path": "vida/config/policies",
        "owned_write_scopes": ["vida/config/policies"]
    });
    let mut command_json = serde_json::to_value(command).expect("command should serialize");
    command_json["trusted_owned_path"] = json!("vida/config/policies");
    command_json["trusted_owned_write_scopes"] = json!(["vida/config/policies"]);
    let command: VidaCommandEnvelope =
        serde_json::from_value(command_json).expect("command should deserialize");

    assert_eq!(command.trusted_owned_path, None);
    assert!(command.trusted_owned_write_scopes.is_empty());

    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.blockers[0].code,
        "operation_owned_write_scope_denied"
    );
}

#[test]
fn command_pipeline_uses_cedar_for_client_kind_denial() {
    let mut command = authorized_task_apply_envelope();
    command.client_kind = VidaClientKind::Cli;

    let response = InProcessVidaClient::new_ready().execute(command);

    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(response.blockers[0].code, "operation_client_kind_denied");
}

#[test]
fn command_pipeline_denies_task_apply_without_trusted_owned_write_scope_before_claim_policy() {
    let mut command = authorized_task_apply_envelope();
    command.trusted_owned_path = None;
    command.trusted_owned_write_scopes.clear();
    command.claim_kind = Some(vida_contracts::VidaClaimKind::SharedRead);

    let response = InProcessVidaClient::new_ready().execute(command);

    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.blockers[0].code,
        "operation_owned_write_scope_denied"
    );
}

#[test]
fn command_pipeline_denies_task_apply_without_trusted_owned_write_scope_before_project_policy() {
    let mut command = authorized_task_apply_envelope();
    command.trusted_owned_path = None;
    command.trusted_owned_write_scopes.clear();
    command.payload["resource_project_id"] = json!("foreign-project");

    let response = InProcessVidaClient::new_ready().execute(command);

    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.blockers[0].code,
        "operation_owned_write_scope_denied"
    );
}

#[test]
fn command_pipeline_denies_task_apply_with_payload_out_of_scope_write_evidence() {
    let mut command = authorized_task_apply_envelope();
    command.trusted_owned_path = Some("crates/vida/src/main.rs".to_string());
    command.trusted_owned_write_scopes = vec!["crates/taskflow-authority".to_string()];

    let response = InProcessVidaClient::new_ready().execute(command);

    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.blockers[0].code,
        "operation_owned_write_scope_denied"
    );
}

fn authorized_task_apply_envelope() -> VidaCommandEnvelope {
    let mut command = envelope(operations::TASK_APPLY);
    command.client_kind = VidaClientKind::HostAgent;
    command.project_ref = Some(local_project_ref());
    command.idempotency_key = Some(VidaIdempotencyKey("task-apply-idem".to_string()));
    command.apply_token = Some(VidaApplyToken("task-apply-token".to_string()));
    command.trusted_owned_path = Some("crates/taskflow-authority".to_string());
    command.trusted_owned_write_scopes = vec!["crates/taskflow-authority".to_string()];
    command
}

fn authorized_wizard_plan_envelope(operation: &str) -> VidaCommandEnvelope {
    let mut command = envelope(operation);
    command.project_ref = Some(local_project_ref());
    command.trusted_owned_path = Some("crates/vida-contracts".to_string());
    command.trusted_owned_write_scopes = vec!["crates/vida-contracts".to_string()];
    command
}

#[test]
fn inprocess_client_uses_local_runtime_handler_for_service_reads() {
    for operation in [
        operations::SERVICE_HELLO,
        operations::SERVICE_STATUS,
        operations::SERVICE_CAPABILITIES,
        operations::SERVICE_ENDPOINT_STATUS,
        operations::SERVICE_LIFECYCLE_PLAN,
        operations::SERVICE_LIFECYCLE_STATUS,
        operations::EVENTS_SINCE,
        operations::SESSION_RESOLVE,
    ] {
        let response = execute(operation);
        assert_eq!(response.status, VidaResponseStatus::Pass, "{operation}");
        assert!(response.error.is_none(), "{operation}");
        let result = response.result.expect("local runtime result");
        assert!(
            !result.to_string().contains("fixture"),
            "local runtime result must not expose fixture payload markers"
        );
    }
}

#[test]
fn local_runtime_status_reflects_current_project_root() {
    let response = execute(operations::SERVICE_STATUS);
    let result = response.result.expect("status result");

    assert_eq!(result["service"], "vida");
    assert_eq!(result["status"], "ready");
    assert_eq!(result["session"]["status"], "active");
    assert_eq!(result["event_cursor"]["current"], "local-runtime-current");
    assert!(result["project_root"]
        .as_str()
        .expect("project root")
        .contains("vida"));
}

#[test]
fn local_runtime_capabilities_expose_engine_negotiation_snapshot() {
    let response = execute(operations::SERVICE_CAPABILITIES);
    let result = response.result.expect("capability result");
    let engine = &result["engine_capabilities"];

    assert_eq!(engine["contract_version"], "vida-runtime-engine-v1");
    assert_eq!(engine["engine_id"], "vida-runtime-local");
    assert_eq!(engine["engine_kind"], "local_redb_effectum");
    assert!(engine["capabilities"]
        .as_array()
        .expect("capability rows")
        .iter()
        .any(|entry| entry["capability"] == "jobs" && entry["supported"] == true));
    assert!(engine["capabilities"]
        .as_array()
        .expect("capability rows")
        .iter()
        .any(|entry| entry["capability"] == "durable_timers"
            && entry["supported"] == false
            && entry["blocker_code"] == "unsupported_engine_capability"));
}

#[test]
fn local_runtime_project_registry_and_status_use_current_worktree() {
    let list = execute(operations::PROJECT_REGISTRY_LIST)
        .result
        .expect("registry list result");
    let projects = list["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["service_binding_status"], "local_inprocess");
    assert_eq!(projects[0]["health"]["source"], "local_runtime_projection");

    let status = execute(operations::PROJECT_STATUS)
        .result
        .expect("project status result");
    assert_eq!(status["status"], "ready");
    assert_eq!(status["service_binding_status"], "local_inprocess");
    assert_eq!(
        status["actor"]["mutation_intent_serialization"]["apply_execution_supported"],
        false
    );
}

#[test]
fn local_runtime_project_scoped_reads_require_matching_project_ref() {
    for operation in [
        operations::PROJECT_RESOLVE,
        operations::PROJECT_STATUS,
        operations::WIZARD_SCHEMA_GET,
        operations::RECEIPTS_GET,
        operations::MATERIALIZATION_MANIFEST_GET,
        operations::MATERIALIZATION_DRIFT_CLASSIFY,
        operations::MATERIALIZATION_UPDATE_PLAN,
        operations::MATERIALIZATION_RECEIPTS_LIST,
        operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET,
    ] {
        let missing_ref = InProcessVidaClient::new_ready().execute(envelope(operation));
        assert_eq!(
            missing_ref.status,
            VidaResponseStatus::Blocked,
            "{operation}"
        );
        assert_eq!(
            missing_ref.blockers[0].code,
            "operation_project_ref_required"
        );
        assert!(missing_ref.result.is_none(), "{operation}");

        let mut unknown_project = envelope(operation);
        unknown_project.project_ref = Some(VidaProjectRef::ProjectId {
            project_id: VidaProjectId("definitely-not-local".to_string()),
        });
        let unknown_ref = InProcessVidaClient::new_ready().execute(unknown_project);
        assert_eq!(
            unknown_ref.status,
            VidaResponseStatus::Blocked,
            "{operation}"
        );
        assert_eq!(unknown_ref.blockers[0].code, "project_not_registered");
        assert!(unknown_ref.result.is_none(), "{operation}");
    }
}

#[test]
fn jobs_get_host_bridge_request_path_is_scoped_to_state_root() {
    let root = unique_test_project_root("vida-client-host-bridge-path");
    let state_root = root.join(".vida/data/state");
    let request_path = state_root.join("host-tool-bridge/requests/request-1.json");
    let retry_policy = RetryPolicy::default();
    let max_attempts = retry_policy.max_attempts;
    std::fs::create_dir_all(request_path.parent().expect("request parent"))
        .expect("create request parent");
    let request = current_host_bridge_request_fixture(&state_root, &request_path, max_attempts);
    let snapshot = HostBridgeRequestJobSnapshot::from_request(&request)
        .expect("current host bridge request snapshot");
    assert_eq!(snapshot.attempt_count, max_attempts);
    let expected_plan = plan_host_bridge_request_job(&snapshot, &retry_policy);
    assert_eq!(
        expected_plan.lifecycle,
        DurableJobLifecycle::DeadLettered,
        "retry policy max_attempts must deadletter the exhausted request"
    );
    std::fs::write(
        &request_path,
        serde_json::to_string(&request).expect("serialize request"),
    )
    .expect("write request");

    let job_client = LocalRuntimeVidaClient::with_job_journal_path(
        root.clone(),
        root.join("missing-operational-journal.redb"),
    );
    let mut command = envelope(operations::JOBS_GET);
    command.payload = json!({
        "job_id": "req-scoped",
        "host_bridge_request_path": request_path.display().to_string()
    });

    let job = job_client
        .execute(command)
        .result
        .expect("host bridge job result");
    assert_eq!(
        job["status"],
        format!("{:?}", expected_plan.lifecycle).to_ascii_lowercase()
    );
    assert_eq!(job["authority"], "host_bridge_request");
    assert_eq!(job["job"]["outbox_id"], "req-scoped");
}

#[test]
fn jobs_get_host_bridge_request_path_rejects_files_outside_state_root() {
    let root = unique_test_project_root("vida-client-host-bridge-outside-path");
    let outside_dir = unique_test_project_root("vida-client-host-bridge-outside-file");
    let outside_path = outside_dir.join("request.json");
    std::fs::write(
        &outside_path,
        serde_json::to_string(&json!({
            "schema_version": 1,
            "status": "failed",
            "request_id": "outside-secret",
            "run_id": "run-outside",
            "task_id": "task-outside",
            "dispatch_target": "worker",
            "attempt_count": 3,
            "failure_reason": "SECRET_FROM_OUTSIDE_FILE"
        }))
        .expect("serialize outside request"),
    )
    .expect("write outside request");

    let job_client = LocalRuntimeVidaClient::with_job_journal_path(
        root.clone(),
        root.join("missing-operational-journal.redb"),
    );
    let mut command = envelope(operations::JOBS_GET);
    command.payload = json!({
        "job_id": "outside-job",
        "host_bridge_request_path": outside_path.display().to_string()
    });

    let job = job_client
        .execute(command)
        .result
        .expect("fallback job result");
    assert_eq!(job["authority"], "redb_outbox");
    assert_eq!(job["status"], "unavailable");
    assert!(!job.to_string().contains("SECRET_FROM_OUTSIDE_FILE"));
    assert!(!job.to_string().contains("outside-secret"));
}

fn unique_test_project_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(root.join(".vida/data/state")).expect("create state root");
    root
}

#[test]
fn local_runtime_wizard_jobs_and_receipts_are_read_projection_routes() {
    let schema = execute(operations::WIZARD_SCHEMA_GET)
        .result
        .expect("wizard schema");
    assert_eq!(schema["schema_id"], "vida.project_init.local_runtime.v1");
    assert_eq!(schema["apply_supported"], false);

    let session = InProcessVidaClient::new_ready()
        .execute(authorized_wizard_plan_envelope(
            operations::WIZARD_SESSION_START,
        ))
        .result
        .expect("wizard session");
    assert_eq!(session["wizard_session"]["step"], "draft");
    assert_eq!(session["apply_supported"], false);

    let job_client = LocalRuntimeVidaClient::with_job_journal_path(
        std::env::current_dir().expect("cwd"),
        persisted_outbox_journal("vida-client"),
    );
    let job = job_client
        .execute(envelope(operations::JOBS_GET))
        .result
        .expect("job result");
    assert_eq!(job["status"], "retryable");
    assert_eq!(job["authority"], "redb_outbox");
    assert_eq!(job["runner"], "effectum");
    assert_eq!(job["job"]["trace"][0]["detail"], "redb_outbox");
    assert_eq!(job["job"]["next_action"], "schedule_retry_from_redb_outbox");
    assert_eq!(job["source"], "local_runtime_projection");

    let mut host_bridge_envelope = envelope(operations::JOBS_GET);
    host_bridge_envelope.payload = json!({
        "host_bridge_request": {
            "request_id": "req-conformance",
            "run_id": "run-conformance",
            "status": "failed",
            "attempt_count": 3,
            "failure_reason": "adapter exhausted"
        }
    });
    let host_bridge_job = job_client
        .execute(host_bridge_envelope)
        .result
        .expect("host bridge job result");
    assert_eq!(host_bridge_job["status"], "deadlettered");
    assert_eq!(host_bridge_job["authority"], "host_bridge_request");
    assert_eq!(host_bridge_job["runner"], "parent_host_adapter");
    assert_eq!(
        host_bridge_job["job"]["job_type"],
        "vida.host_bridge.adapter_request"
    );
    assert_eq!(
        host_bridge_job["job"]["blocker"]["code"],
        "host_bridge_adapter_request_dead_letter"
    );

    let receipt = execute(operations::RECEIPTS_GET)
        .result
        .expect("receipt result");
    assert_eq!(receipt["receipt_scope"], "project");
    assert_eq!(receipt["receipts"][0]["kind"], "local_runtime_projection");
}

fn persisted_outbox_journal(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("vida-{label}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create journal fixture dir");
    let path = dir.join("journal.redb");
    let mut journal = RedbOperationalJournal::create(&path).expect("create redb journal");
    journal
        .append(JournalAppendRequest {
            stream_id: VidaStreamRef("stream-1".to_string()),
            expected_stream_version: Some(VidaStreamVersion(0)),
            command_id: VidaCommandRef("command-1".to_string()),
            idempotency_key: VidaIdempotencyKey("idem-1".to_string()),
            causation_id: Some(VidaCommandRef("command-1".to_string())),
            correlation_id: Some("correlation-1".to_string()),
            events: vec![VidaDomainEventEnvelope {
                schema_id: VidaSchemaId("schema.task.updated".to_string()),
                event_version: VidaSchemaVersion(1),
                event_id: VidaEventRef("event-1".to_string()),
                command_id: Some(VidaCommandRef("command-1".to_string())),
                causation_id: Some(VidaCommandRef("command-1".to_string())),
                stream_id: VidaStreamRef("stream-1".to_string()),
                stream_version: VidaStreamVersion(1),
                aggregate_id: VidaAggregateRef("task-1".to_string()),
                occurred_at: VidaTimestamp("2026-06-23T00:00:00Z".to_string()),
                payload: serde_json::json!({ "stream_version": 1 }),
                trace: serde_json::json!({ "correlation_id": "correlation-1" }),
            }],
            effect_intents: vec![VidaEffectIntent {
                effect_id: VidaEffectRef("effect-1".to_string()),
                operation: VidaOperation("vida.effect.dispatch".to_string()),
                command_id: VidaCommandRef("command-1".to_string()),
                stream_id: VidaStreamRef("stream-1".to_string()),
                payload: serde_json::json!({ "effect_id": "effect-1" }),
            }],
        })
        .expect("append effect intent");
    let claimed = journal.claim_outbox_batch("effectum-worker-1", 1);
    journal
        .mark_outbox_failed(&claimed[0].outbox_id, "transport failure".to_string())
        .expect("mark failed");
    path
}

#[test]
fn unsupported_operations_still_return_structured_problem() {
    let response = execute("vida.local-runtime.unknown");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    let problem = response.error.expect("unsupported operation problem");
    assert_eq!(problem.code, "unsupported_operation");
    assert_eq!(response.blockers[0].code, "operation_not_registered");
}
