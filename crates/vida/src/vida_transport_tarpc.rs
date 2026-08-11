use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use futures::prelude::*;
use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, ToNsName,
    tokio::{Stream as LocalSocketStream, prelude::*},
};
use serde_json::json;
use tarpc::serde_transport;
use tarpc::server::{BaseChannel, Channel};
use tarpc::tokio_serde::formats::Json;
use tarpc::tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tarpc::{client, context};
use vida_contracts::{VidaCommandEnvelope, VidaCommandResponse};

use crate::vida_client::VidaClient;
use crate::vida_client_inprocess::InProcessVidaClient;

type TransportResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tarpc::service]
trait VidaEnvelopeRpc {
    async fn execute(envelope: VidaCommandEnvelope) -> VidaCommandResponse;
}

#[derive(Clone)]
struct VidaEnvelopeServer {
    client: InProcessVidaClient,
}

impl VidaEnvelopeServer {
    fn new_ready() -> Self {
        Self {
            client: InProcessVidaClient::new_ready(),
        }
    }
}

impl VidaEnvelopeRpc for VidaEnvelopeServer {
    async fn execute(
        self,
        _: context::Context,
        envelope: VidaCommandEnvelope,
    ) -> VidaCommandResponse {
        self.client.execute(envelope)
    }
}

pub(crate) struct TarpcLocalIpcVidaClient {
    client: VidaEnvelopeRpcClient,
    _server_task: tokio::task::JoinHandle<()>,
}

impl TarpcLocalIpcVidaClient {
    pub(crate) async fn connect_ready_local_channel() -> TransportResult<Self> {
        let (client_transport, server_transport) = tarpc::transport::channel::unbounded();
        let server_task = tokio::spawn(
            BaseChannel::with_defaults(server_transport)
                .execute(VidaEnvelopeServer::new_ready().serve())
                .for_each(|request| async move {
                    request.await;
                }),
        );
        let client =
            VidaEnvelopeRpcClient::new(client::Config::default(), client_transport).spawn();

        Ok(Self {
            client,
            _server_task: server_task,
        })
    }

    pub(crate) async fn connect_ready_local_socket() -> TransportResult<Self> {
        let socket_name = unique_socket_name();
        let listener_name = socket_name.as_str().to_ns_name::<GenericNamespaced>()?;
        let listener = ListenerOptions::new()
            .name(listener_name)
            .try_overwrite(true)
            .create_tokio()?;

        let server = VidaEnvelopeServer::new_ready();
        let server_task = tokio::spawn(async move {
            if let Ok(connection) = listener.accept().await {
                let framed = LengthDelimitedCodec::builder().new_framed(connection);
                let transport = serde_transport::new(framed, Json::default());
                BaseChannel::with_defaults(transport)
                    .execute(server.serve())
                    .for_each(|request| async move {
                        request.await;
                    })
                    .await;
            }
        });
        tokio::task::yield_now().await;

        let client_name = socket_name.as_str().to_ns_name::<GenericNamespaced>()?;
        let connection = LocalSocketStream::connect(client_name).await?;
        let framed = LengthDelimitedCodec::builder().new_framed(connection);
        let transport = serde_transport::new(framed, Json::default());
        let client = VidaEnvelopeRpcClient::new(client::Config::default(), transport).spawn();

        Ok(Self {
            client,
            _server_task: server_task,
        })
    }

    pub(crate) async fn execute(
        &self,
        envelope: VidaCommandEnvelope,
    ) -> TransportResult<VidaCommandResponse> {
        Ok(self.client.execute(context::current(), envelope).await?)
    }
}

fn unique_socket_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("vida-tarpc-smoke-{}-{nanos}.sock", std::process::id())
}

pub(crate) fn local_socket_endpoint_metadata() -> serde_json::Value {
    json!({
        "transport": "tarpc",
        "framing": "length_delimited_json",
        "preferred_local_ipc": [
            "windows_named_pipe",
            "unix_domain_socket"
        ],
        "fallback": {
            "kind": "loopback_tcp",
            "allowed": true,
            "requires_token": true,
            "token_value_exposed": false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{TarpcLocalIpcVidaClient, local_socket_endpoint_metadata};
    use vida_contracts::{
        VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaClaimKind,
        VidaClientKind, VidaCommandEnvelope, VidaOperation, VidaRequestId, VidaResponseStatus,
        VidaSessionId, operations,
    };

    fn envelope(operation: &str) -> VidaCommandEnvelope {
        VidaCommandEnvelope {
            schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
            protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
            operation: VidaOperation(operation.to_string()),
            session_id: VidaSessionId("session-tarpc-test".to_string()),
            request_id: VidaRequestId("request-tarpc-test".to_string()),
            command_id: None,
            causation_id: None,
            expected_stream_version: None,
            consistency: None,
            deadline: None,
            client_kind: VidaClientKind::Cli,
            project_ref: None,
            claim_kind: Some(VidaClaimKind::SharedRead),
            trusted_owned_path: None,
            trusted_owned_write_scopes: Vec::new(),
            payload: serde_json::json!({}),
            correlation: None,
            idempotency_key: None,
            apply_token: None,
        }
    }

    #[test]
    fn endpoint_metadata_preserves_tarpc_framing_and_fallback_contract() {
        let metadata = local_socket_endpoint_metadata();

        assert_eq!(metadata["transport"], "tarpc");
        assert_eq!(metadata["framing"], "length_delimited_json");
        assert_eq!(
            metadata["preferred_local_ipc"],
            serde_json::json!(["windows_named_pipe", "unix_domain_socket"])
        );
        assert_eq!(metadata["fallback"]["kind"], "loopback_tcp");
        assert_eq!(metadata["fallback"]["allowed"], true);
        assert_eq!(metadata["fallback"]["requires_token"], true);
        assert_eq!(metadata["fallback"]["token_value_exposed"], false);
    }

    #[tokio::test]
    async fn in_process_tarpc_channel_preserves_request_and_ready_response() {
        let client = TarpcLocalIpcVidaClient::connect_ready_local_channel()
            .await
            .expect("in-process tarpc channel should connect");

        let response = client
            .execute(envelope(operations::SERVICE_HELLO))
            .await
            .expect("in-process tarpc request should execute");

        assert_eq!(response.request_id.0, "request-tarpc-test");
        assert_eq!(response.status, VidaResponseStatus::Pass);
        assert_eq!(response.result.as_ref().unwrap()["service"], "vida");
    }
}
