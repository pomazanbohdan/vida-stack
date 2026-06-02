use std::error::Error;

use futures::prelude::*;
use tarpc::server::{BaseChannel, Channel};
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

    pub(crate) async fn execute(
        &self,
        envelope: VidaCommandEnvelope,
    ) -> TransportResult<VidaCommandResponse> {
        Ok(self.client.execute(context::current(), envelope).await?)
    }
}
