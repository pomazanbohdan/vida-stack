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
    pub(crate) fn execute(&self, envelope: VidaCommandEnvelope) -> VidaCommandResponse {
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
