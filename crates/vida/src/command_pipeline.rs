use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use tower::Service;
use vida_contracts::{
    operation_spec, VidaBlocker, VidaCommandEnvelope, VidaCommandResponse, VidaOperationSpec,
    VidaProblem, VidaProblemSeverity, VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
};

use crate::vida_client::VidaClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandPipelineLayer {
    Trace,
    Deadline,
    SchemaProtocol,
    OperationLookup,
    ProjectRouting,
    AuthorizationAdmission,
    Idempotency,
    Concurrency,
    Handler,
    ResponseMapping,
}

pub(crate) const COMMAND_PIPELINE_LAYER_ORDER: [CommandPipelineLayer; 10] = [
    CommandPipelineLayer::Trace,
    CommandPipelineLayer::Deadline,
    CommandPipelineLayer::SchemaProtocol,
    CommandPipelineLayer::OperationLookup,
    CommandPipelineLayer::ProjectRouting,
    CommandPipelineLayer::AuthorizationAdmission,
    CommandPipelineLayer::Idempotency,
    CommandPipelineLayer::Concurrency,
    CommandPipelineLayer::Handler,
    CommandPipelineLayer::ResponseMapping,
];

#[derive(Debug, Clone)]
pub(crate) struct VidaCommandPipeline<H> {
    handler: H,
}

impl<H> VidaCommandPipeline<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self { handler }
    }

    pub(crate) fn layer_order() -> &'static [CommandPipelineLayer] {
        &COMMAND_PIPELINE_LAYER_ORDER
    }

    pub(crate) fn trace_names() -> Vec<&'static str> {
        COMMAND_PIPELINE_LAYER_ORDER
            .iter()
            .map(|layer| layer_name(*layer))
            .collect()
    }
}

impl<H: VidaClient> VidaCommandPipeline<H> {
    pub(crate) fn execute(&self, mut envelope: VidaCommandEnvelope) -> VidaCommandResponse {
        let mut trace = Vec::with_capacity(COMMAND_PIPELINE_LAYER_ORDER.len());

        trace.push(CommandPipelineLayer::Trace);
        trace.push(CommandPipelineLayer::Deadline);

        trace.push(CommandPipelineLayer::SchemaProtocol);
        if envelope.schema_version != VIDA_CONTRACTS_SCHEMA_VERSION {
            return blocked_response(
                envelope,
                "schema_version_mismatch",
                "Command schema version mismatch",
                "schema_version",
                "Use the current VIDA contracts schema version.",
            );
        }
        if envelope.protocol_version != VIDA_COMMAND_PROTOCOL_VERSION {
            return blocked_response(
                envelope,
                "protocol_version_mismatch",
                "Command protocol version mismatch",
                "protocol_version",
                "Use the current VIDA command protocol version.",
            );
        }
        if let Err(problem) = envelope.canonicalize_operation_alias() {
            let blocker_code = problem.blocker_code;
            let message = problem.message;
            return blocked_response(
                envelope,
                &blocker_code,
                "Legacy operation alias is ambiguous",
                "operation",
                &message,
            );
        }

        trace.push(CommandPipelineLayer::OperationLookup);
        let Some(spec) = operation_spec(&envelope.operation.0) else {
            return crate::vida_client::unsupported_operation_response(&envelope);
        };

        trace.push(CommandPipelineLayer::ProjectRouting);

        trace.push(CommandPipelineLayer::AuthorizationAdmission);
        if let Some(response) = authorization_blocker(&envelope, &spec) {
            return response;
        }

        trace.push(CommandPipelineLayer::Idempotency);
        if spec.requires_idempotency_key && envelope.idempotency_key.is_none() {
            return blocked_response(
                envelope,
                "idempotency_key_required",
                "Idempotency key required",
                "idempotency_key",
                "Provide an idempotency key for mutation operations.",
            );
        }
        if spec.requires_apply_token && envelope.apply_token.is_none() {
            return blocked_response(
                envelope,
                "apply_token_required",
                "Apply token required",
                "apply_token",
                "Provide an apply token for apply or admin operations.",
            );
        }

        trace.push(CommandPipelineLayer::Concurrency);
        trace.push(CommandPipelineLayer::Handler);
        let response = self.handler.execute(envelope);

        trace.push(CommandPipelineLayer::ResponseMapping);
        debug_assert_eq!(trace.as_slice(), COMMAND_PIPELINE_LAYER_ORDER);
        response
    }
}

impl<H: VidaClient + Clone> Service<VidaCommandEnvelope> for VidaCommandPipeline<H> {
    type Response = VidaCommandResponse;
    type Error = std::convert::Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, envelope: VidaCommandEnvelope) -> Self::Future {
        ready(Ok(self.execute(envelope)))
    }
}

fn authorization_blocker(
    envelope: &VidaCommandEnvelope,
    spec: &VidaOperationSpec,
) -> Option<VidaCommandResponse> {
    if !spec.allowed_client_kinds.contains(&envelope.client_kind) {
        return Some(blocked_response(
            envelope.clone(),
            "client_kind_not_allowed",
            "Client kind is not allowed",
            "client_kind",
            "Use an allowed client kind for this operation.",
        ));
    }
    if envelope.claim_kind.as_ref() != Some(&spec.required_claim) {
        return Some(blocked_response(
            envelope.clone(),
            "claim_kind_required",
            "Required claim kind missing",
            "claim_kind",
            "Use the claim kind required by operation metadata.",
        ));
    }
    None
}

fn blocked_response(
    envelope: VidaCommandEnvelope,
    code: &str,
    title: &str,
    scope: &str,
    next_action: &str,
) -> VidaCommandResponse {
    let problem = VidaProblem {
        problem_type: format!("https://vida.dev/problems/{code}"),
        title: title.to_string(),
        detail: format!("{title}: {scope}."),
        code: code.to_string(),
        severity: VidaProblemSeverity::Error,
        retryable: false,
        blockers: vec![VidaBlocker {
            code: code.to_string(),
            scope: Some(scope.to_string()),
            next_actions: vec![next_action.to_string()],
        }],
        remediation: vec![next_action.to_string()],
        instance: None,
        related_receipt: None,
    };
    crate::vida_client::problem_response(&envelope, problem)
}

fn layer_name(layer: CommandPipelineLayer) -> &'static str {
    match layer {
        CommandPipelineLayer::Trace => "trace",
        CommandPipelineLayer::Deadline => "deadline",
        CommandPipelineLayer::SchemaProtocol => "schema_protocol",
        CommandPipelineLayer::OperationLookup => "operation_lookup",
        CommandPipelineLayer::ProjectRouting => "project_routing",
        CommandPipelineLayer::AuthorizationAdmission => "authorization_admission",
        CommandPipelineLayer::Idempotency => "idempotency",
        CommandPipelineLayer::Concurrency => "concurrency",
        CommandPipelineLayer::Handler => "handler",
        CommandPipelineLayer::ResponseMapping => "response_mapping",
    }
}

#[cfg(test)]
mod tests {
    use super::VidaCommandPipeline;
    use crate::vida_client::{pass_response, VidaClient};
    use serde_json::json;
    use vida_contracts::{
        operations, VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaCommandResponse,
        VidaOperation, VidaProjectRef, VidaRequestId, VidaResponseStatus, VidaSessionId,
        VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
    };

    #[derive(Debug, Clone)]
    struct EchoOperationClient;

    impl VidaClient for EchoOperationClient {
        fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
            let alias_receipt = envelope
                .correlation
                .as_ref()
                .and_then(|value| value.get("operation_alias_receipt"))
                .cloned();
            pass_response(
                &envelope,
                json!({
                    "operation": envelope.operation.0,
                    "alias_receipt": alias_receipt
                }),
            )
        }
    }

    fn service_status_envelope(operation: &str) -> VidaCommandEnvelope {
        VidaCommandEnvelope {
            schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
            protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
            operation: VidaOperation(operation.to_string()),
            session_id: VidaSessionId("session-1".to_string()),
            request_id: VidaRequestId("request-1".to_string()),
            command_id: None,
            causation_id: None,
            expected_stream_version: None,
            consistency: None,
            deadline: None,
            client_kind: VidaClientKind::Cli,
            project_ref: None::<VidaProjectRef>,
            claim_kind: Some(VidaClaimKind::SharedRead),
            payload: json!({}),
            correlation: None,
            idempotency_key: None,
            apply_token: None,
        }
    }

    #[test]
    fn pipeline_routes_direct_legacy_alias_to_canonical_handler_operation() {
        let pipeline = VidaCommandPipeline::new(EchoOperationClient);
        let canonical = pipeline.execute(service_status_envelope(operations::SERVICE_STATUS));
        let alias = pipeline.execute(service_status_envelope("service.status"));

        assert_eq!(canonical.status, VidaResponseStatus::Pass);
        assert_eq!(alias.status, VidaResponseStatus::Pass);
        assert_eq!(
            alias.result.as_ref().expect("alias result")["operation"],
            canonical.result.as_ref().expect("canonical result")["operation"]
        );
        assert_eq!(
            alias.result.as_ref().expect("alias result")["alias_receipt"]["alias"],
            "service.status"
        );
    }

    #[test]
    fn pipeline_blocks_ambiguous_legacy_alias_before_handler() {
        let pipeline = VidaCommandPipeline::new(EchoOperationClient);
        let response = pipeline.execute(VidaCommandEnvelope {
            operation: VidaOperation("status".to_string()),
            claim_kind: Some(VidaClaimKind::SharedRead),
            idempotency_key: None,
            apply_token: None,
            ..service_status_envelope(operations::SERVICE_STATUS)
        });

        assert_eq!(response.status, VidaResponseStatus::Blocked);
        assert_eq!(
            response.blockers[0].code,
            "ambiguous_legacy_operation_alias"
        );
    }
}
