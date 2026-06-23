use vida_contracts::{
    RuntimeEngine, RuntimeEngineCapabilities, RuntimeEngineCapability,
    RuntimeEngineCapabilitySupport, RuntimeEngineError, RuntimeEngineHealth, RuntimeEngineResult,
    RuntimeQueryRequest, RuntimeWatchPlan, RuntimeWatchRequest,
    VIDA_RUNTIME_ENGINE_CONTRACT_VERSION, VidaCommandEnvelope, VidaCommandResponse, VidaOperation,
    VidaResponseStatus,
};

pub const LOCAL_ENGINE_ID: &str = "vida-runtime-local";
pub const FAKE_ENGINE_ID: &str = "vida-runtime-fake";
pub const UNSUPPORTED_ENGINE_CAPABILITY: &str = "unsupported_engine_capability";

#[derive(Debug, Clone, Default)]
pub struct LocalRuntimeEngine;

#[derive(Debug, Clone, Default)]
pub struct FakeRuntimeEngine;

pub fn local_runtime_capabilities() -> RuntimeEngineCapabilities {
    RuntimeEngineCapabilities {
        contract_version: VIDA_RUNTIME_ENGINE_CONTRACT_VERSION.to_string(),
        engine_id: LOCAL_ENGINE_ID.to_string(),
        engine_kind: "local_redb_effectum".to_string(),
        capabilities: vec![
            supported(RuntimeEngineCapability::Jobs, "redb_outbox_effectum"),
            supported(
                RuntimeEngineCapability::EventExport,
                "redb_operational_journal",
            ),
            supported(
                RuntimeEngineCapability::StrongReads,
                "local_projection_snapshot",
            ),
            supported(RuntimeEngineCapability::OfflineMode, "local_state_store"),
            unsupported(RuntimeEngineCapability::DurableTimers),
            unsupported(RuntimeEngineCapability::KeyedSerialization),
            unsupported(RuntimeEngineCapability::Signals),
        ],
    }
}

pub fn fake_runtime_capabilities() -> RuntimeEngineCapabilities {
    RuntimeEngineCapabilities {
        contract_version: VIDA_RUNTIME_ENGINE_CONTRACT_VERSION.to_string(),
        engine_id: FAKE_ENGINE_ID.to_string(),
        engine_kind: "fake_conformance".to_string(),
        capabilities: vec![
            supported(RuntimeEngineCapability::Jobs, "in_memory_fake"),
            supported(RuntimeEngineCapability::EventExport, "in_memory_fake"),
            supported(RuntimeEngineCapability::StrongReads, "in_memory_fake"),
            unsupported(RuntimeEngineCapability::DurableTimers),
            unsupported(RuntimeEngineCapability::KeyedSerialization),
            unsupported(RuntimeEngineCapability::Signals),
            unsupported(RuntimeEngineCapability::OfflineMode),
        ],
    }
}

impl RuntimeEngine for LocalRuntimeEngine {
    fn capabilities(&self) -> RuntimeEngineCapabilities {
        local_runtime_capabilities()
    }

    fn health(&self) -> RuntimeEngineHealth {
        RuntimeEngineHealth {
            engine_id: LOCAL_ENGINE_ID.to_string(),
            status: "ready".to_string(),
            blocker_codes: Vec::new(),
        }
    }

    fn execute(&self, envelope: VidaCommandEnvelope) -> RuntimeEngineResult<VidaCommandResponse> {
        Ok(accepted_projection_response(envelope))
    }

    fn query(&self, request: RuntimeQueryRequest) -> RuntimeEngineResult<serde_json::Value> {
        Ok(serde_json::json!({
            "engine_id": LOCAL_ENGINE_ID,
            "operation": request.operation.0,
            "query_mode": "local_projection_snapshot"
        }))
    }

    fn watch(&self, request: RuntimeWatchRequest) -> RuntimeEngineResult<RuntimeWatchPlan> {
        watch_plan(&self.capabilities(), request)
    }
}

impl RuntimeEngine for FakeRuntimeEngine {
    fn capabilities(&self) -> RuntimeEngineCapabilities {
        fake_runtime_capabilities()
    }

    fn health(&self) -> RuntimeEngineHealth {
        RuntimeEngineHealth {
            engine_id: FAKE_ENGINE_ID.to_string(),
            status: "ready".to_string(),
            blocker_codes: Vec::new(),
        }
    }

    fn execute(&self, envelope: VidaCommandEnvelope) -> RuntimeEngineResult<VidaCommandResponse> {
        Ok(accepted_projection_response(envelope))
    }

    fn query(&self, request: RuntimeQueryRequest) -> RuntimeEngineResult<serde_json::Value> {
        Ok(serde_json::json!({
            "engine_id": FAKE_ENGINE_ID,
            "operation": request.operation.0,
            "query_mode": "fake_projection_snapshot"
        }))
    }

    fn watch(&self, request: RuntimeWatchRequest) -> RuntimeEngineResult<RuntimeWatchPlan> {
        watch_plan(&self.capabilities(), request)
    }
}

fn watch_plan(
    capabilities: &RuntimeEngineCapabilities,
    request: RuntimeWatchRequest,
) -> RuntimeEngineResult<RuntimeWatchPlan> {
    if !capabilities.supports(request.required_capability) {
        return Err(RuntimeEngineError::UnsupportedCapability {
            capability: request.required_capability,
            blocker_code: UNSUPPORTED_ENGINE_CAPABILITY.to_string(),
            remediation:
                "Select an engine that advertises the requested capability before starting a watch."
                    .to_string(),
        });
    }
    Ok(RuntimeWatchPlan {
        stream_kind: "domain_event_export".to_string(),
        replayable: true,
        cursor: request.cursor,
    })
}

fn accepted_projection_response(envelope: VidaCommandEnvelope) -> VidaCommandResponse {
    VidaCommandResponse {
        request_id: envelope.request_id,
        status: VidaResponseStatus::Accepted,
        result: Some(serde_json::json!({
            "operation": envelope.operation.0,
            "engine_boundary": "runtime_engine"
        })),
        error: None,
        receipt_ref: None,
        job_ref: None,
        blockers: Vec::new(),
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
        blocker_code: Some(UNSUPPORTED_ENGINE_CAPABILITY.to_string()),
    }
}

pub fn unsupported_operation_error(operation: impl Into<String>) -> RuntimeEngineError {
    RuntimeEngineError::UnsupportedOperation {
        operation: VidaOperation(operation.into()),
        blocker_code: "unsupported_engine_operation".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use vida_contracts::{
        RuntimeEngineCapability, RuntimeQueryRequest, RuntimeWatchRequest,
        VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION, VidaClientKind,
        VidaIdempotencyKey, VidaOperation, VidaRequestId, VidaSessionId,
    };

    use super::*;

    #[test]
    fn local_engine_capability_response_snapshot_is_stable() {
        let snapshot = serde_json::to_value(local_runtime_capabilities()).expect("snapshot");

        assert_eq!(snapshot["contract_version"], "vida-runtime-engine-v1");
        assert_eq!(snapshot["engine_id"], LOCAL_ENGINE_ID);
        assert_eq!(snapshot["engine_kind"], "local_redb_effectum");
        assert!(
            snapshot["capabilities"]
                .as_array()
                .expect("array")
                .iter()
                .any(|entry| entry["capability"] == "jobs" && entry["supported"] == true)
        );
        assert!(
            snapshot["capabilities"]
                .as_array()
                .expect("array")
                .iter()
                .any(|entry| entry["capability"] == "durable_timers"
                    && entry["blocker_code"] == UNSUPPORTED_ENGINE_CAPABILITY)
        );
    }

    #[test]
    fn fake_engine_conformance_matches_runtime_engine_contract() {
        let engine = FakeRuntimeEngine;

        assert!(
            engine
                .capabilities()
                .supports(RuntimeEngineCapability::Jobs)
        );
        assert_eq!(engine.health().status, "ready");

        let response = engine
            .execute(envelope("vida.fake.execute"))
            .expect("execute");
        assert_eq!(response.status, VidaResponseStatus::Accepted);

        let query = engine
            .query(RuntimeQueryRequest {
                operation: VidaOperation("vida.fake.query".to_string()),
                payload: serde_json::json!({}),
            })
            .expect("query");
        assert_eq!(query["engine_id"], FAKE_ENGINE_ID);

        let watch = engine
            .watch(RuntimeWatchRequest {
                cursor: None,
                required_capability: RuntimeEngineCapability::Jobs,
            })
            .expect("watch");
        assert!(watch.replayable);
    }

    #[test]
    fn unsupported_capability_fails_explicitly() {
        let error = LocalRuntimeEngine
            .watch(RuntimeWatchRequest {
                cursor: None,
                required_capability: RuntimeEngineCapability::DurableTimers,
            })
            .expect_err("durable timers not supported locally yet");

        assert!(matches!(
            error,
            RuntimeEngineError::UnsupportedCapability { .. }
        ));
        assert_eq!(
            LocalRuntimeEngine
                .capabilities()
                .unsupported(RuntimeEngineCapability::DurableTimers),
            Some(UNSUPPORTED_ENGINE_CAPABILITY)
        );
    }

    fn envelope(operation: &str) -> VidaCommandEnvelope {
        VidaCommandEnvelope {
            schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
            protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
            operation: VidaOperation(operation.to_string()),
            session_id: VidaSessionId("fake-engine-session".to_string()),
            request_id: VidaRequestId("fake-engine-request".to_string()),
            command_id: None,
            causation_id: None,
            expected_stream_version: None,
            consistency: None,
            deadline: None,
            client_kind: VidaClientKind::Service,
            project_ref: None,
            claim_kind: None,
            payload: serde_json::json!({}),
            correlation: None,
            idempotency_key: Some(VidaIdempotencyKey("fake-engine-idem".to_string())),
            apply_token: None,
        }
    }
}
