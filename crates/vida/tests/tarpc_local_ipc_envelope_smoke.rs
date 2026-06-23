#[path = "../src/command_pipeline.rs"]
mod command_pipeline;
#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;
#[path = "../src/vida_transport_tarpc.rs"]
mod vida_transport_tarpc;

use serde_json::json;
use vida_client::VidaClient;
use vida_client_inprocess::InProcessVidaClient;
use vida_contracts::{
    operation_spec, operations, VidaClientKind, VidaCommandEnvelope, VidaIdempotencyKey,
    VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId, VidaResponseStatus, VidaSessionId,
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};
use vida_transport_tarpc::TarpcLocalIpcVidaClient;

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("tarpc-smoke-session".to_string()),
        request_id: VidaRequestId(format!("tarpc-smoke-{operation}")),
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: VidaClientKind::Service,
        project_ref: None,
        claim_kind: operation_spec(operation).map(|spec| spec.required_claim),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("tarpc-smoke-idem-{operation}"))),
        apply_token: None,
    }
}

fn envelope_with_project_ref(operation: &str, project_ref: VidaProjectRef) -> VidaCommandEnvelope {
    let mut envelope = envelope(operation);
    envelope.project_ref = Some(project_ref);
    envelope
}

fn envelope_with_project_ref_and_payload(
    operation: &str,
    project_ref: VidaProjectRef,
    payload: serde_json::Value,
) -> VidaCommandEnvelope {
    let mut envelope = envelope_with_project_ref(operation, project_ref);
    envelope.payload = payload;
    envelope
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tarpc_local_channel_envelope_smoke_carries_service_read_operations() {
    let tarpc_client = TarpcLocalIpcVidaClient::connect_ready_local_channel()
        .await
        .expect("tarpc local channel client");
    assert_tarpc_read_operations_match_in_process(tarpc_client).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tarpc_local_socket_ipc_envelope_smoke_carries_service_read_operations() {
    let tarpc_client = TarpcLocalIpcVidaClient::connect_ready_local_socket()
        .await
        .expect("tarpc local socket client");
    assert_tarpc_read_operations_match_in_process(tarpc_client).await;
}

async fn assert_tarpc_read_operations_match_in_process(tarpc_client: TarpcLocalIpcVidaClient) {
    let in_process = InProcessVidaClient::new_ready();

    for operation in [
        operations::SERVICE_HELLO,
        operations::SERVICE_STATUS,
        operations::EVENTS_SINCE,
    ] {
        let expected = in_process.execute(envelope(operation));
        let actual = tarpc_client
            .execute(envelope(operation))
            .await
            .expect("tarpc envelope response");

        assert_eq!(actual, expected);
        assert_eq!(actual.status, VidaResponseStatus::Pass);
        assert!(actual.result.is_some());
        assert!(actual.error.is_none());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tarpc_interprocess_transport_matches_inprocess_conformance_matrix() {
    let tarpc_client = TarpcLocalIpcVidaClient::connect_ready_local_socket()
        .await
        .expect("tarpc local socket client");
    let in_process = InProcessVidaClient::new_ready();
    let project_ref = VidaProjectRef::ProjectId {
        project_id: VidaProjectId("vida-stack".to_string()),
    };
    let requests = vec![
        envelope(operations::SERVICE_HELLO),
        envelope(operations::SERVICE_STATUS),
        envelope(operations::SERVICE_CAPABILITIES),
        envelope(operations::SERVICE_ENDPOINT_STATUS),
        envelope(operations::EVENTS_SINCE),
        envelope(operations::SESSION_RESOLVE),
        envelope_with_project_ref(operations::PROJECT_RESOLVE, project_ref.clone()),
        envelope_with_project_ref(operations::PROJECT_STATUS, project_ref.clone()),
        envelope_with_project_ref_and_payload(
            operations::WIZARD_SCHEMA_GET,
            project_ref.clone(),
            json!({ "wizard_kind": "project_init" }),
        ),
        envelope_with_project_ref_and_payload(
            operations::WIZARD_SESSION_START,
            project_ref.clone(),
            json!({ "wizard_kind": "project_init" }),
        ),
        envelope_with_project_ref(
            operations::MATERIALIZATION_MANIFEST_GET,
            project_ref.clone(),
        ),
        envelope_with_project_ref(
            operations::MATERIALIZATION_DRIFT_CLASSIFY,
            project_ref.clone(),
        ),
        envelope_with_project_ref_and_payload(
            operations::MATERIALIZATION_UPDATE_PLAN,
            project_ref.clone(),
            json!({ "mode": "safe_update" }),
        ),
        envelope_with_project_ref(
            operations::MATERIALIZATION_RECEIPTS_LIST,
            project_ref.clone(),
        ),
        envelope_with_project_ref(
            operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET,
            project_ref,
        ),
    ];

    for request in requests {
        let operation = request.operation.0.clone();
        let expected = in_process.execute(request.clone());
        let actual = tarpc_client
            .execute(request)
            .await
            .unwrap_or_else(|error| panic!("{operation} tarpc response: {error}"));

        assert_eq!(actual, expected, "{operation} should match in-process");
        assert_eq!(actual.status, VidaResponseStatus::Pass, "{operation}");
        assert!(actual.error.is_none(), "{operation}");
    }

    let endpoints = tarpc_client
        .execute(envelope(operations::SERVICE_ENDPOINT_STATUS))
        .await
        .expect("endpoint status over tarpc")
        .result
        .expect("endpoint status result");
    let endpoint_rows = endpoints["endpoints"].as_array().expect("endpoint rows");
    assert!(endpoint_rows.iter().any(|row| {
        row["operation"] == operations::SERVICE_LIFECYCLE_APPLY
            && row["posture"] == "apply"
            && row["requires_apply_token"] == true
    }));
    assert!(endpoint_rows.iter().any(|row| {
        row["operation"] == operations::ORCHESTRATION_CONTROL_PLANE_SUMMARY_GET
            && row["scope"] == "project"
            && row["posture"] == "read_only"
    }));
}

#[test]
fn tarpc_endpoint_metadata_prefers_local_ipc_and_keeps_tcp_token_secret() {
    let metadata = vida_transport_tarpc::local_socket_endpoint_metadata();

    assert!(metadata["preferred_local_ipc"]
        .as_array()
        .expect("preferred local IPC entries")
        .iter()
        .any(|entry| entry == "windows_named_pipe"));
    assert!(metadata["preferred_local_ipc"]
        .as_array()
        .expect("preferred local IPC entries")
        .iter()
        .any(|entry| entry == "unix_domain_socket"));
    assert_eq!(metadata["fallback"]["kind"], "loopback_tcp");
    assert_eq!(metadata["fallback"]["requires_token"], true);
    assert_eq!(metadata["fallback"]["token_value_exposed"], false);
}
