#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_fixture.rs"]
mod vida_client_fixture;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;

use serde_json::json;
use vida_client::VidaClient;
use vida_client_fixture::FixtureVidaClient;
use vida_client_inprocess::InProcessVidaClient;
use vida_contracts::{
    mvp_operation_registry, operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope,
    VidaCommandResponse, VidaIdempotencyKey, VidaOperation, VidaOperationPosture,
    VidaOperationScope, VidaRequestId, VidaResponseStatus, VidaSessionId,
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("test-session".to_string()),
        request_id: VidaRequestId(format!("request-{operation}")),
        client_kind: VidaClientKind::Cli,
        project_ref: None,
        claim_kind: Some(VidaClaimKind::Observe),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("idem-{operation}"))),
        apply_token: None,
    }
}

fn assert_same_response(operation: &str) -> VidaCommandResponse {
    let fixture = FixtureVidaClient::new_ready();
    let in_process = InProcessVidaClient::new_ready();
    let fixture_response = fixture.execute(envelope(operation));
    let in_process_response = in_process.execute(envelope(operation));
    assert_eq!(fixture_response, in_process_response);
    fixture_response
}

#[test]
fn vida_client_fixture_and_inprocess_match_service_read_operations() {
    for operation in [
        operations::SERVICE_HELLO,
        operations::SERVICE_STATUS,
        operations::SERVICE_CAPABILITIES,
        operations::SERVICE_ENDPOINT_STATUS,
        operations::EVENTS_SINCE,
        operations::SESSION_RESOLVE,
    ] {
        let response = assert_same_response(operation);
        assert_eq!(response.status, VidaResponseStatus::Pass);
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}

#[test]
fn service_status_reports_session_and_event_cursor() {
    let response = assert_same_response(operations::SERVICE_STATUS);
    let result = response.result.expect("status result");

    assert_eq!(result["service"], "vida");
    assert_eq!(result["status"], "ready");
    assert_eq!(result["session"]["status"], "active");
    assert_eq!(result["event_cursor"]["current"], "fixture-cursor-1");
}

#[test]
fn service_capabilities_and_endpoints_are_read_only() {
    let capabilities = assert_same_response(operations::SERVICE_CAPABILITIES)
        .result
        .expect("capabilities result");
    assert_eq!(capabilities["service"], "vida");
    assert_eq!(capabilities["mutation_apply_supported"], false);
    assert!(capabilities["capabilities"]
        .as_array()
        .expect("capabilities should be array")
        .iter()
        .any(|capability| capability == "read_status"));

    let endpoints = assert_same_response(operations::SERVICE_ENDPOINT_STATUS)
        .result
        .expect("endpoint status result");
    let endpoint_rows = endpoints["endpoints"]
        .as_array()
        .expect("endpoints should be array");
    assert!(endpoint_rows.iter().any(|row| {
        row["operation"] == operations::SERVICE_HELLO && row["posture"] == "read_only"
    }));
    assert!(endpoint_rows
        .iter()
        .all(|row| row["posture"] != "apply" && row["posture"] != "admin"));
}

#[test]
fn events_since_reports_current_cursor() {
    let response = assert_same_response(operations::EVENTS_SINCE);
    let result = response.result.expect("events result");

    assert_eq!(result["current_cursor"], "fixture-cursor-1");
    assert_eq!(result["events"][0]["cursor"], "fixture-cursor-1");
}

#[test]
fn session_resolve_reports_active_session_status() {
    let response = assert_same_response(operations::SESSION_RESOLVE);
    let result = response.result.expect("session result");

    assert_eq!(result["session_id"], "test-session");
    assert_eq!(result["status"], "active");
    assert_eq!(result["service_status"], "ready");
}

#[test]
fn service_home_registry_exposes_no_mutation_capable_apply_operations() {
    let service_specs: Vec<_> = mvp_operation_registry()
        .into_iter()
        .filter(|spec| spec.scope == VidaOperationScope::Service)
        .collect();
    assert!(!service_specs.is_empty());
    assert!(service_specs.iter().all(|spec| {
        spec.posture != VidaOperationPosture::Apply
            && spec.posture != VidaOperationPosture::Admin
            && !spec.requires_apply_token
    }));

    let response = assert_same_response("vida.service.apply");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    assert_eq!(
        response.error.expect("unsupported apply operation").code,
        "unsupported_operation"
    );
}

#[test]
fn vida_client_unsupported_operation_returns_structured_problem() {
    let response = assert_same_response("vida.unknown.operation");
    assert_eq!(response.status, VidaResponseStatus::Blocked);
    let problem = response.error.expect("unsupported operation problem");
    assert_eq!(problem.code, "unsupported_operation");
    assert_eq!(response.blockers.len(), 1);
    assert_eq!(response.blockers[0].code, "operation_not_registered");
}
