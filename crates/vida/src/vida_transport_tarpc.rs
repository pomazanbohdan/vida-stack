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
