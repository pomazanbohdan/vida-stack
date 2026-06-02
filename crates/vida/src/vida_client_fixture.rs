use crate::vida_client::{pass_response, unsupported_operation_response, VidaClient};
use serde_json::json;
use vida_contracts::{
    mvp_operation_registry, operations, VidaCommandEnvelope, VidaCommandResponse, VidaEvent,
    VidaEventCursor, VidaRequestId, VidaSessionId,
};

#[derive(Debug, Clone)]
pub(crate) struct FixtureVidaClient {
    service_status: String,
    session_status: String,
    current_cursor: VidaEventCursor,
    events: Vec<VidaEvent>,
}

impl FixtureVidaClient {
    pub(crate) fn new_ready() -> Self {
        let session_id = VidaSessionId("fixture-session".to_string());
        let request_id = VidaRequestId("fixture-request".to_string());
        let current_cursor = VidaEventCursor("fixture-cursor-1".to_string());
        Self {
            service_status: "ready".to_string(),
            session_status: "active".to_string(),
            current_cursor: current_cursor.clone(),
            events: vec![VidaEvent {
                event_id: "fixture-event-1".to_string(),
                request_id,
                session_id,
                project_id: None,
                job_id: None,
                kind: "service.ready".to_string(),
                payload: json!({ "status": "ready" }),
                cursor: current_cursor,
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
                "status": self.service_status,
                "session": {
                    "session_id": envelope.session_id,
                    "status": self.session_status
                },
                "event_cursor": {
                    "current": self.current_cursor
                }
            }),
        )
    }

    fn capabilities(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "mutation_apply_supported": false,
                "capabilities": [
                    "read_status",
                    "read_events"
                ]
            }),
        )
    }

    fn endpoint_status(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        let endpoints: Vec<_> = mvp_operation_registry()
            .into_iter()
            .filter(|spec| matches!(spec.scope, vida_contracts::VidaOperationScope::Service))
            .map(|spec| {
                json!({
                    "operation": spec.operation.0,
                    "scope": spec.scope,
                    "posture": spec.posture,
                    "requires_project_ref": spec.requires_project_ref,
                    "requires_apply_token": spec.requires_apply_token,
                    "required_capabilities": spec.required_capabilities
                })
            })
            .collect();
        pass_response(
            envelope,
            json!({
                "service": "vida",
                "status": self.service_status,
                "endpoints": endpoints
            }),
        )
    }

    fn events_since(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "current_cursor": self.current_cursor,
                "events": self.events
            }),
        )
    }

    fn session_resolve(&self, envelope: &VidaCommandEnvelope) -> VidaCommandResponse {
        pass_response(
            envelope,
            json!({
                "session_id": envelope.session_id,
                "status": self.session_status,
                "service_status": self.service_status,
                "event_cursor": self.current_cursor
            }),
        )
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
            operations::SERVICE_CAPABILITIES => self.capabilities(&envelope),
            operations::SERVICE_ENDPOINT_STATUS => self.endpoint_status(&envelope),
            operations::EVENTS_SINCE => self.events_since(&envelope),
            operations::SESSION_RESOLVE => self.session_resolve(&envelope),
            _ => unsupported_operation_response(&envelope),
        }
    }
}
