#[path = "../src/command_pipeline.rs"]
mod command_pipeline;
#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;

use serde_json::json;
use vida_client::VidaClient;
use vida_client_inprocess::{InProcessVidaClient, LocalRuntimeVidaClient};
use vida_contracts::{
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaApplyToken, VidaClientKind,
    VidaCommandEnvelope, VidaIdempotencyKey, VidaOperation, VidaRequestId, VidaResponseStatus,
    VidaSessionId, operation_spec, operations,
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
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("idem-{operation}"))),
        apply_token: None,
    }
}

fn execute(operation: &str) -> vida_contracts::VidaCommandResponse {
    InProcessVidaClient::new_ready().execute(envelope(operation))
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
    assert_eq!(response.blockers[0].code, "idempotency_key_required");
}

#[test]
fn command_pipeline_blocks_apply_operation_without_apply_token_after_idempotency() {
    let mut command = envelope(operations::SERVICE_LIFECYCLE_APPLY);
    command.client_kind = VidaClientKind::Service;
    command.apply_token = None;
    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(response.blockers[0].code, "apply_token_required");
}

#[test]
fn command_pipeline_reaches_handler_when_apply_token_is_present() {
    let mut command = envelope(operations::SERVICE_LIFECYCLE_APPLY);
    command.client_kind = VidaClientKind::Service;
    command.apply_token = Some(VidaApplyToken("test-apply-token".to_string()));
    let response = InProcessVidaClient::new_ready().execute(command);
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(response.blockers[0].code, "operation_not_registered");
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
    assert!(
        result["project_root"]
            .as_str()
            .expect("project root")
            .contains("vida")
    );
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
fn local_runtime_wizard_jobs_and_receipts_are_read_projection_routes() {
    let schema = execute(operations::WIZARD_SCHEMA_GET)
        .result
        .expect("wizard schema");
    assert_eq!(schema["schema_id"], "vida.project_init.local_runtime.v1");
    assert_eq!(schema["apply_supported"], false);

    let session = execute(operations::WIZARD_SESSION_START)
        .result
        .expect("wizard session");
    assert_eq!(session["wizard_session"]["step"], "draft");
    assert_eq!(session["apply_supported"], false);

    let job = execute(operations::JOBS_GET).result.expect("job result");
    assert_eq!(job["status"], "completed");
    assert_eq!(job["source"], "local_runtime_projection");

    let receipt = execute(operations::RECEIPTS_GET)
        .result
        .expect("receipt result");
    assert_eq!(receipt["receipt_scope"], "project");
    assert_eq!(receipt["receipts"][0]["kind"], "local_runtime_projection");
}

#[test]
fn unsupported_operations_still_return_structured_problem() {
    let response = execute("vida.local-runtime.unknown");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    let problem = response.error.expect("unsupported operation problem");
    assert_eq!(problem.code, "unsupported_operation");
    assert_eq!(response.blockers[0].code, "operation_not_registered");
}
