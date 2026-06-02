use crate::vida_client::{pass_response, unsupported_operation_response, VidaClient};
use serde_json::json;
use vida_contracts::{
    operations, VidaCommandEnvelope, VidaCommandResponse, VidaEvent, VidaEventCursor,
    VidaRequestId, VidaSessionId,
};

#[derive(Debug, Clone)]
pub(crate) struct FixtureVidaClient {
    service_status: String,
    events: Vec<VidaEvent>,
}

impl FixtureVidaClient {
    pub(crate) fn new_ready() -> Self {
        let session_id = VidaSessionId("fixture-session".to_string());
        let request_id = VidaRequestId("fixture-request".to_string());
        Self {
            service_status: "ready".to_string(),
            events: vec![VidaEvent {
                event_id: "fixture-event-1".to_string(),
                request_id,
                session_id,
                project_id: None,
                job_id: None,
                kind: "service.ready".to_string(),
                payload: json!({ "status": "ready" }),
                cursor: VidaEventCursor("fixture-cursor-1".to_string()),
            }],
        }
    }

    fn hello(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "protocol_version": envelope.protocol_version,
                "schema_version": envelope.schema_version
            }),
        )
    }

    fn status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status
            }),
        )
    }

    fn events_since(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(envelope, json!({ "events": self.events }))
    }
}

impl Default for FixtureVidaClient {
    fn default() -> Self {
        Self::new_ready()
    }
}

impl VidaClient for FixtureVidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        match envelope.operation.0.as_str() {
            operations::SERVICE_HELLO => self.hello(&envelope),
            operations::SERVICE_STATUS => self.status(&envelope),
            operations::EVENTS_SINCE => self.events_since(&envelope),
            _ => unsupported_operation_response(&envelope),
        }
    }
}
