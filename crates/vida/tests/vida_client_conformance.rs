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
    operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaCommandResponse,
    VidaIdempotencyKey, VidaOperation, VidaRequestId, VidaResponseStatus, VidaSessionId,
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
        operations::EVENTS_SINCE,
    ] {
        let response = assert_same_response(operation);
        assert_eq!(response.status, VidaResponseStatus::Pass);
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
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
