use crate::command_pipeline::VidaCommandPipeline;
use crate::vida_client::VidaClient;
use crate::vida_client_fixture::FixtureVidaClient;
use vida_contracts::{VidaCommandEnvelope, VidaCommandResponse};

#[derive(Debug, Clone)]
pub(crate) struct InProcessVidaClient {
    pipeline: VidaCommandPipeline<FixtureVidaClient>,
}

impl InProcessVidaClient {
    pub(crate) fn new_ready() -> Self {
        Self {
            pipeline: VidaCommandPipeline::new(FixtureVidaClient::new_ready()),
        }
    }
}

impl VidaClient for InProcessVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        self.pipeline.execute(envelope)
    }
}
