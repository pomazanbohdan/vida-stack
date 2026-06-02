use crate::vida_client::VidaClient;
use crate::vida_client_fixture::FixtureVidaClient;
use vida_contracts::{VidaCommandEnvelope, VidaCommandResponse};

#[derive(Debug, Clone, Default)]
pub(crate) struct InProcessVidaClient {
    fixture: FixtureVidaClient,
}

impl InProcessVidaClient {
    pub(crate) fn new_ready() -> Self {
        Self {
            fixture: FixtureVidaClient::new_ready(),
        }
    }
}

impl VidaClient for InProcessVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        self.fixture.execute(envelope)
    }
}
