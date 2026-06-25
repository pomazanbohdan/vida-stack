use serde::{Deserialize, Serialize};
use vida_contracts::{
    RuntimeEngine, RuntimeEngineCapabilities, RuntimeEngineCapability,
    RuntimeEngineCapabilitySupport, RuntimeEngineError, RuntimeEngineHealth, RuntimeEngineResult,
    RuntimeQueryRequest, RuntimeWatchPlan, RuntimeWatchRequest,
    VIDA_RUNTIME_ENGINE_CONTRACT_VERSION, VidaCommandEnvelope, VidaCommandResponse, VidaOperation,
    VidaResponseStatus, runtime_envelope_schema_bundle_json,
};

pub const RESTATE_ADAPTER_ENGINE_ID: &str = "vida-runtime-restate-adapter-prototype";
pub const RESTATE_ADAPTER_UNSUPPORTED_OPERATION: &str = "restate_adapter_unsupported_operation";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestateInvocationPrototype {
    pub service_key: String,
    pub handler: String,
    pub request_id: String,
    pub idempotency_key: Option<String>,
    pub project_ref: Option<String>,
    pub payload: serde_json::Value,
    pub correlation: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct RestateAdapterPrototype;

pub fn restate_adapter_capabilities() -> RuntimeEngineCapabilities {
    RuntimeEngineCapabilities {
        contract_version: VIDA_RUNTIME_ENGINE_CONTRACT_VERSION.to_string(),
        engine_id: RESTATE_ADAPTER_ENGINE_ID.to_string(),
        engine_kind: "external_restate_adapter_prototype".to_string(),
        capabilities: vec![
            supported(RuntimeEngineCapability::Jobs, "restate_invocation"),
            supported(
                RuntimeEngineCapability::DurableTimers,
                "restate_timer_model",
            ),
            supported(
                RuntimeEngineCapability::KeyedSerialization,
                "session_id_service_key",
            ),
            supported(RuntimeEngineCapability::Signals, "restate_signal_model"),
            supported(RuntimeEngineCapability::EventExport, "vida_schema_registry"),
            supported(RuntimeEngineCapability::StrongReads, "vida_query_contract"),
            unsupported(RuntimeEngineCapability::OfflineMode),
        ],
    }
}

pub fn map_command_to_restate_invocation(
    envelope: &VidaCommandEnvelope,
) -> RestateInvocationPrototype {
    RestateInvocationPrototype {
        service_key: envelope.session_id.0.clone(),
        handler: envelope.operation.0.replace('.', "/"),
        request_id: envelope.request_id.0.clone(),
        idempotency_key: envelope.idempotency_key.as_ref().map(|key| key.0.clone()),
        project_ref: envelope.project_ref.as_ref().map(|project| match project {
            vida_contracts::VidaProjectRef::ProjectId { project_id } => project_id.0.clone(),
            vida_contracts::VidaProjectRef::RegistryEntry { registry_entry_id } => {
                registry_entry_id.clone()
            }
            vida_contracts::VidaProjectRef::RootPath { root_path } => root_path.clone(),
        }),
        payload: envelope.payload.clone(),
        correlation: envelope.correlation.clone(),
    }
}

pub fn restate_adapter_schema_bundle_json() -> serde_json::Value {
    runtime_envelope_schema_bundle_json()
}

impl RuntimeEngine for RestateAdapterPrototype {
    fn capabilities(&self) -> RuntimeEngineCapabilities {
        restate_adapter_capabilities()
    }

    fn health(&self) -> RuntimeEngineHealth {
        RuntimeEngineHealth {
            engine_id: RESTATE_ADAPTER_ENGINE_ID.to_string(),
            status: "prototype_ready_no_cutover".to_string(),
            blocker_codes: Vec::new(),
        }
    }

    fn execute(&self, envelope: VidaCommandEnvelope) -> RuntimeEngineResult<VidaCommandResponse> {
        let invocation = map_command_to_restate_invocation(&envelope);
        Ok(VidaCommandResponse {
            request_id: envelope.request_id,
            status: VidaResponseStatus::Accepted,
            result: Some(serde_json::json!({
                "engine_boundary": "runtime_engine",
                "adapter": RESTATE_ADAPTER_ENGINE_ID,
                "handler": invocation.handler,
                "service_key": invocation.service_key
            })),
            error: None,
            receipt_ref: None,
            job_ref: None,
            blockers: Vec::new(),
        })
    }

    fn query(&self, request: RuntimeQueryRequest) -> RuntimeEngineResult<serde_json::Value> {
        Ok(serde_json::json!({
            "engine_id": RESTATE_ADAPTER_ENGINE_ID,
            "operation": request.operation.0,
            "query_mode": "prototype_restate_query_contract"
        }))
    }

    fn watch(&self, request: RuntimeWatchRequest) -> RuntimeEngineResult<RuntimeWatchPlan> {
        if !self.capabilities().supports(request.required_capability) {
            return Err(RuntimeEngineError::UnsupportedCapability {
                capability: request.required_capability,
                blocker_code: "unsupported_restate_adapter_capability".to_string(),
                remediation:
                    "Select an external runtime adapter that advertises the requested capability."
                        .to_string(),
            });
        }
        Ok(RuntimeWatchPlan {
            stream_kind: "prototype_restate_event_export".to_string(),
            replayable: true,
            cursor: request.cursor,
        })
    }
}

fn supported(capability: RuntimeEngineCapability, mode: &str) -> RuntimeEngineCapabilitySupport {
    RuntimeEngineCapabilitySupport {
        capability,
        supported: true,
        mode: mode.to_string(),
        blocker_code: None,
    }
}

fn unsupported(capability: RuntimeEngineCapability) -> RuntimeEngineCapabilitySupport {
    RuntimeEngineCapabilitySupport {
        capability,
        supported: false,
        mode: "unsupported".to_string(),
        blocker_code: Some("unsupported_restate_adapter_capability".to_string()),
    }
}

pub fn unsupported_operation_error(operation: impl Into<String>) -> RuntimeEngineError {
    RuntimeEngineError::UnsupportedOperation {
        operation: VidaOperation(operation.into()),
        blocker_code: RESTATE_ADAPTER_UNSUPPORTED_OPERATION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vida_contracts::{
        RuntimeWatchRequest, VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
        VidaClientKind, VidaIdempotencyKey, VidaOperation, VidaProjectId, VidaProjectRef,
        VidaRequestId, VidaSessionId,
    };

    #[test]
    fn maps_command_envelope_to_restate_invocation_without_sdk_types() {
        let envelope = envelope("vida.task.apply");

        let invocation = map_command_to_restate_invocation(&envelope);

        assert_eq!(invocation.service_key, "session-a");
        assert_eq!(invocation.handler, "vida/task/apply");
        assert_eq!(invocation.request_id, "request-a");
        assert_eq!(invocation.idempotency_key.as_deref(), Some("idem-a"));
        assert_eq!(invocation.project_ref.as_deref(), Some("project-a"));
        assert_eq!(invocation.payload["task_id"], "task-a");
        assert_eq!(
            invocation.correlation.as_ref().unwrap()["trace_id"],
            "trace-a"
        );
    }

    #[test]
    fn adapter_engine_executes_through_runtime_engine_contract() {
        let engine = RestateAdapterPrototype;

        assert!(
            engine
                .capabilities()
                .supports(RuntimeEngineCapability::KeyedSerialization)
        );
        assert!(
            engine
                .capabilities()
                .supports(RuntimeEngineCapability::DurableTimers)
        );
        assert_eq!(engine.health().status, "prototype_ready_no_cutover");

        let response = engine.execute(envelope("vida.task.apply")).unwrap();

        assert_eq!(response.status, VidaResponseStatus::Accepted);
        assert_eq!(
            response.result.unwrap()["adapter"],
            RESTATE_ADAPTER_ENGINE_ID
        );
    }

    #[test]
    fn adapter_schema_bundle_equals_canonical_vida_contracts_bundle() {
        assert_eq!(
            restate_adapter_schema_bundle_json(),
            runtime_envelope_schema_bundle_json()
        );
    }

    #[test]
    fn unsupported_capability_fails_closed() {
        let error = RestateAdapterPrototype
            .watch(RuntimeWatchRequest {
                cursor: None,
                required_capability: RuntimeEngineCapability::OfflineMode,
            })
            .expect_err("offline mode is intentionally unsupported");

        assert!(matches!(
            error,
            RuntimeEngineError::UnsupportedCapability { .. }
        ));
    }

    #[test]
    fn restate_prototype_passes_shared_conformance_suite() {
        let report = vida_test_support::engine_conformance::verify_runtime_engine_conformance(
            "restate-prototype",
            &RestateAdapterPrototype,
        )
        .unwrap();

        assert_eq!(report.failed_count(), 0);
        let supported = report.supported_capabilities();
        for capability in ["jobs", "durable_timers", "keyed_serialization", "signals"] {
            assert!(supported.contains(&capability.to_string()));
        }
    }

    fn envelope(operation: &str) -> VidaCommandEnvelope {
        VidaCommandEnvelope {
            schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
            protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
            operation: VidaOperation(operation.to_string()),
            session_id: VidaSessionId("session-a".to_string()),
            request_id: VidaRequestId("request-a".to_string()),
            command_id: None,
            causation_id: None,
            expected_stream_version: None,
            consistency: None,
            deadline: None,
            client_kind: VidaClientKind::Service,
            project_ref: Some(VidaProjectRef::ProjectId {
                project_id: VidaProjectId("project-a".to_string()),
            }),
            claim_kind: None,
            payload: serde_json::json!({ "task_id": "task-a" }),
            correlation: Some(serde_json::json!({ "trace_id": "trace-a" })),
            idempotency_key: Some(VidaIdempotencyKey("idem-a".to_string())),
            apply_token: None,
        }
    }
}
