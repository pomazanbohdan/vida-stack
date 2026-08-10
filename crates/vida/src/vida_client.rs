use vida_contracts::{
    VidaCommandEnvelope, VidaCommandResponse, VidaProblem, VidaResponseStatus,
    unsupported_operation_problem,
};

pub(crate) trait VidaClient {
    fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse;
}

pub(crate) fn pass_response(
    envelope: &VidaCommandEnvelope,
    result: serde_json::Value,
) -> VidaCommandResponse {
    VidaCommandResponse {
        request_id: envelope.request_id.clone(),
        status: VidaResponseStatus::Pass,
        result: Some(result),
        error: None,
        receipt_ref: None,
        job_ref: None,
        blockers: Vec::new(),
    }
}

pub(crate) fn problem_response(
    envelope: &VidaCommandEnvelope,
    problem: VidaProblem,
) -> VidaCommandResponse {
    VidaCommandResponse {
        request_id: envelope.request_id.clone(),
        status: VidaResponseStatus::Blocked,
        result: None,
        blockers: problem.blockers.clone(),
        error: Some(problem),
        receipt_ref: None,
        job_ref: None,
    }
}

pub(crate) fn unsupported_operation_response(
    envelope: &VidaCommandEnvelope,
) -> VidaCommandResponse {
    problem_response(
        envelope,
        unsupported_operation_problem(&envelope.operation.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vida_contracts::{
        VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaClaimKind,
        VidaClientKind, VidaCommandEnvelope, VidaOperation, VidaRequestId, VidaSessionId,
    };

    fn envelope() -> VidaCommandEnvelope {
        VidaCommandEnvelope {
            schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
            protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
            operation: VidaOperation("vida.test.unsupported".to_string()),
            session_id: VidaSessionId("session-test".to_string()),
            request_id: VidaRequestId("request-test".to_string()),
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
    fn response_helpers_preserve_request_and_fail_closed_status_contracts() {
        let envelope = envelope();
        let pass = pass_response(&envelope, serde_json::json!({"ok": true}));

        assert_eq!(pass.request_id, envelope.request_id);
        assert_eq!(pass.status, VidaResponseStatus::Pass);
        assert_eq!(pass.result, Some(serde_json::json!({"ok": true})));
        assert!(pass.error.is_none());
        assert!(pass.blockers.is_empty());

        let blocked = unsupported_operation_response(&envelope);
        assert_eq!(blocked.request_id, envelope.request_id);
        assert_eq!(blocked.status, VidaResponseStatus::Blocked);
        assert_eq!(
            blocked.error.as_ref().map(|problem| problem.code.as_str()),
            Some("unsupported_operation")
        );
        assert_eq!(blocked.blockers[0].code, "operation_not_registered");
    }
}
