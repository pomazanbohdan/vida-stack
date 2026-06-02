#[path = "../src/vida_client.rs"]
mod vida_client;
#[path = "../src/vida_client_fixture.rs"]
mod vida_client_fixture;
#[path = "../src/vida_client_inprocess.rs"]
mod vida_client_inprocess;
#[path = "../src/vida_transport_tarpc.rs"]
mod vida_transport_tarpc;

use serde_json::json;
use vida_client::VidaClient;
use vida_client_inprocess::InProcessVidaClient;
use vida_contracts::{
    operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaIdempotencyKey,
    VidaOperation, VidaRequestId, VidaResponseStatus, VidaSessionId, VIDA_COMMAND_PROTOCOL_VERSION,
    VIDA_CONTRACTS_SCHEMA_VERSION,
};
use vida_transport_tarpc::TarpcLocalIpcVidaClient;

fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("tarpc-smoke-session".to_string()),
        request_id: VidaRequestId(format!("tarpc-smoke-{operation}")),
        client_kind: VidaClientKind::Service,
        project_ref: None,
        claim_kind: Some(VidaClaimKind::Observe),
        payload: json!({}),
        correlation: None,
        idempotency_key: Some(VidaIdempotencyKey(format!("tarpc-smoke-idem-{operation}"))),
        apply_token: None,
    }
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
