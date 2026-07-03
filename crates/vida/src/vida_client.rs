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
