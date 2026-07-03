use std::{
    future::{Ready, ready},
    task::{Context, Poll},
};

use taskflow_authority::operation_authorization::{
    OperationAuthorizationDecision, OperationAuthorizationInput, authorize_operation,
};
use tower::Service;
use vida_contracts::{
    VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaBlocker, VidaClaimKind,
    VidaCommandEnvelope, VidaCommandResponse, VidaOperationSpec, VidaProblem, VidaProblemSeverity,
    VidaProjectId, VidaProjectRef, operation_spec,
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
    let decision = authorize_operation(&OperationAuthorizationInput {
        session_id: envelope.session_id.0.clone(),
        project_id: project_id_from_ref(envelope.project_ref.as_ref()),
        client_kind: envelope.client_kind.clone(),
        claim_kind: envelope
            .claim_kind
            .clone()
            .unwrap_or(VidaClaimKind::Observe),
        capability: spec
            .required_capabilities
            .first()
            .cloned()
            .expect("registered operation should declare at least one capability"),
        operation: spec.clone(),
        resource_project_id: string_payload_field(&envelope.payload, "resource_project_id")
            .map(VidaProjectId)
            .or_else(|| project_id_from_ref(envelope.project_ref.as_ref())),
        owned_path: envelope.trusted_owned_path.clone(),
        owned_write_scopes: envelope.trusted_owned_write_scopes.clone(),
        idempotency_key_present: envelope.idempotency_key.is_some(),
        apply_token_present: envelope.apply_token.is_some(),
    });
    (!decision.allowed).then(|| authorization_problem_response(envelope.clone(), decision))
}

fn authorization_problem_response(
    envelope: VidaCommandEnvelope,
    decision: OperationAuthorizationDecision,
) -> VidaCommandResponse {
    let code = decision
        .blocker_codes
        .first()
        .cloned()
        .unwrap_or_else(|| "operation_policy_denied".to_string());
    let next_action =
        decision.remediation.first().cloned().unwrap_or_else(|| {
            "Check operation authorization evidence before retrying.".to_string()
        });
    blocked_response(
        envelope,
        &code,
        "Operation authorization denied",
        "operation_authorization",
        &next_action,
    )
}

fn project_id_from_ref(project_ref: Option<&VidaProjectRef>) -> Option<VidaProjectId> {
    match project_ref {
        Some(VidaProjectRef::ProjectId { project_id }) => Some(project_id.clone()),
        _ => None,
    }
}

fn string_payload_field(payload: &serde_json::Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
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
    use crate::vida_client::{VidaClient, pass_response};
    use serde_json::json;
    use vida_contracts::{
        VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaApplyToken,
        VidaClaimKind, VidaClientKind, VidaCommandEnvelope, VidaCommandResponse,
        VidaIdempotencyKey, VidaOperation, VidaProjectId, VidaProjectRef, VidaRequestId,
        VidaResponseStatus, VidaSessionId, operations,
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
            trusted_owned_path: None,
            trusted_owned_write_scopes: Vec::new(),
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
    fn pipeline_ignores_bounded_payload_supplied_owned_write_evidence() {
        let pipeline = VidaCommandPipeline::new(EchoOperationClient);
        let response = pipeline.execute(VidaCommandEnvelope {
            operation: VidaOperation(operations::TASK_APPLY.to_string()),
            client_kind: VidaClientKind::HostAgent,
            project_ref: Some(VidaProjectRef::ProjectId {
                project_id: VidaProjectId("project-1".to_string()),
            }),
            claim_kind: Some(VidaClaimKind::ExclusiveWrite),
            payload: json!({
                "resource_project_id": "project-1",
                "owned_path": "crates/vida/src/main.rs",
                "owned_write_scopes": ["crates/vida"]
            }),
            idempotency_key: Some(VidaIdempotencyKey("idem-1".to_string())),
            apply_token: Some(VidaApplyToken("apply-1".to_string())),
            ..service_status_envelope(operations::SERVICE_STATUS)
        });

        assert_eq!(response.status, VidaResponseStatus::Blocked);
        assert_eq!(
            response.blockers[0].code,
            "operation_owned_write_scope_denied"
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
