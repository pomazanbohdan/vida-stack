//! Shared RuntimeEngine conformance and determinism helpers.

use serde::{Deserialize, Serialize};
use vida_contracts::{
    RuntimeEngine, RuntimeEngineCapability, RuntimeEngineError, RuntimeQueryRequest,
    RuntimeWatchRequest, VIDA_COMMAND_PROTOCOL_VERSION, VIDA_CONTRACTS_SCHEMA_VERSION,
    VIDA_RUNTIME_ENGINE_CONTRACT_VERSION, VidaClientKind, VidaCommandEnvelope, VidaEventCursor,
    VidaIdempotencyKey, VidaOperation, VidaRequestId, VidaResponseStatus, VidaSessionId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineConformanceReport {
    pub engine_label: String,
    pub engine_id: String,
    pub engine_kind: String,
    pub scenario_results: Vec<EngineConformanceScenarioResult>,
    pub deterministic_signature: String,
}

impl EngineConformanceReport {
    pub fn scenario_count(&self) -> usize {
        self.scenario_results.len()
    }

    pub fn failed_count(&self) -> usize {
        self.scenario_results
            .iter()
            .filter(|scenario| scenario.status != "pass")
            .count()
    }

    pub fn supported_capabilities(&self) -> Vec<String> {
        self.scenario_results
            .iter()
            .filter_map(|scenario| {
                scenario
                    .capability
                    .as_ref()
                    .filter(|_| scenario.status == "pass" && scenario.name == "supported_watch")
                    .cloned()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineConformanceScenarioResult {
    pub name: String,
    pub status: String,
    pub capability: Option<String>,
    pub detail: String,
}

pub fn verify_runtime_engine_conformance<E: RuntimeEngine>(
    engine_label: &str,
    engine: &E,
) -> Result<EngineConformanceReport, String> {
    let capabilities = engine.capabilities();
    let health = engine.health();
    let mut scenario_results = Vec::new();

    require(
        capabilities.contract_version == VIDA_RUNTIME_ENGINE_CONTRACT_VERSION,
        "contract_version",
        format!(
            "expected {}, got {}",
            VIDA_RUNTIME_ENGINE_CONTRACT_VERSION, capabilities.contract_version
        ),
        &mut scenario_results,
    );
    require(
        health.engine_id == capabilities.engine_id,
        "health_engine_id",
        format!(
            "health={} capabilities={}",
            health.engine_id, capabilities.engine_id
        ),
        &mut scenario_results,
    );
    require(
        !capabilities.capabilities.is_empty(),
        "capability_advertisement",
        format!(
            "advertised {} capabilities",
            capabilities.capabilities.len()
        ),
        &mut scenario_results,
    );

    for support in &capabilities.capabilities {
        let watch_result = engine.watch(RuntimeWatchRequest {
            cursor: Some(VidaEventCursor("cursor-a".to_string())),
            required_capability: support.capability,
        });
        match (support.supported, watch_result) {
            (true, Ok(plan)) if plan.replayable => scenario_results.push(pass_with_capability(
                "supported_watch",
                support.capability,
                format!("mode={} stream={}", support.mode, plan.stream_kind),
            )),
            (true, Ok(_plan)) => scenario_results.push(fail_with_capability(
                "supported_watch",
                support.capability,
                format!("mode={} returned non-replayable watch plan", support.mode),
            )),
            (true, Err(error)) => scenario_results.push(fail_with_capability(
                "supported_watch",
                support.capability,
                format!("mode={} errored: {error:?}", support.mode),
            )),
            (false, Err(RuntimeEngineError::UnsupportedCapability { .. })) => scenario_results
                .push(pass_with_capability(
                    "unsupported_watch",
                    support.capability,
                    format!("mode={} failed closed", support.mode),
                )),
            (false, Ok(plan)) => scenario_results.push(fail_with_capability(
                "unsupported_watch",
                support.capability,
                format!(
                    "mode={} unexpectedly returned {}",
                    support.mode, plan.stream_kind
                ),
            )),
            (false, Err(error)) => scenario_results.push(fail_with_capability(
                "unsupported_watch",
                support.capability,
                format!("mode={} returned wrong error: {error:?}", support.mode),
            )),
        }
    }

    let first_execute = response_bytes(
        engine
            .execute(envelope("vida.conformance.execute"))
            .map_err(|error| format!("execute failed: {error:?}"))?,
    )?;
    let second_execute = response_bytes(
        engine
            .execute(envelope("vida.conformance.execute"))
            .map_err(|error| format!("execute failed: {error:?}"))?,
    )?;
    require(
        first_execute == second_execute,
        "execute_determinism",
        format!(
            "first={} second={}",
            first_execute.len(),
            second_execute.len()
        ),
        &mut scenario_results,
    );

    let query = RuntimeQueryRequest {
        operation: VidaOperation("vida.conformance.query".to_string()),
        payload: serde_json::json!({"probe":"determinism"}),
    };
    let first_query = json_bytes(
        engine
            .query(query.clone())
            .map_err(|error| format!("query failed: {error:?}"))?,
    )?;
    let second_query = json_bytes(
        engine
            .query(query)
            .map_err(|error| format!("query failed: {error:?}"))?,
    )?;
    require(
        first_query == second_query,
        "query_determinism",
        format!("first={} second={}", first_query.len(), second_query.len()),
        &mut scenario_results,
    );

    let deterministic_signature = format!(
        "execute={};query={}",
        String::from_utf8(first_execute).map_err(|error| error.to_string())?,
        String::from_utf8(first_query).map_err(|error| error.to_string())?
    );

    let report = EngineConformanceReport {
        engine_label: engine_label.to_string(),
        engine_id: capabilities.engine_id,
        engine_kind: capabilities.engine_kind,
        scenario_results,
        deterministic_signature,
    };

    if report.failed_count() == 0 {
        Ok(report)
    } else {
        Err(serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?)
    }
}

fn require(
    condition: bool,
    name: &str,
    detail: String,
    scenario_results: &mut Vec<EngineConformanceScenarioResult>,
) {
    scenario_results.push(EngineConformanceScenarioResult {
        name: name.to_string(),
        status: if condition { "pass" } else { "fail" }.to_string(),
        capability: None,
        detail,
    });
}

fn pass_with_capability(
    name: &str,
    capability: RuntimeEngineCapability,
    detail: String,
) -> EngineConformanceScenarioResult {
    EngineConformanceScenarioResult {
        name: name.to_string(),
        status: "pass".to_string(),
        capability: Some(capability_name(capability).to_string()),
        detail,
    }
}

fn fail_with_capability(
    name: &str,
    capability: RuntimeEngineCapability,
    detail: String,
) -> EngineConformanceScenarioResult {
    EngineConformanceScenarioResult {
        name: name.to_string(),
        status: "fail".to_string(),
        capability: Some(capability_name(capability).to_string()),
        detail,
    }
}

fn capability_name(capability: RuntimeEngineCapability) -> &'static str {
    match capability {
        RuntimeEngineCapability::Jobs => "jobs",
        RuntimeEngineCapability::DurableTimers => "durable_timers",
        RuntimeEngineCapability::KeyedSerialization => "keyed_serialization",
        RuntimeEngineCapability::Signals => "signals",
        RuntimeEngineCapability::EventExport => "event_export",
        RuntimeEngineCapability::StrongReads => "strong_reads",
        RuntimeEngineCapability::OfflineMode => "offline_mode",
    }
}

fn response_bytes(response: vida_contracts::VidaCommandResponse) -> Result<Vec<u8>, String> {
    if response.status != VidaResponseStatus::Accepted {
        return Err(format!(
            "expected accepted response, got {:?}",
            response.status
        ));
    }
    json_bytes(response)
}

fn json_bytes<T: Serialize>(value: T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&value).map_err(|error| error.to_string())
}

pub fn envelope(operation: &str) -> VidaCommandEnvelope {
    VidaCommandEnvelope {
        schema_version: VIDA_CONTRACTS_SCHEMA_VERSION.to_string(),
        protocol_version: VIDA_COMMAND_PROTOCOL_VERSION.to_string(),
        operation: VidaOperation(operation.to_string()),
        session_id: VidaSessionId("conformance-session".to_string()),
        request_id: VidaRequestId("conformance-request".to_string()),
        command_id: None,
        causation_id: None,
        expected_stream_version: None,
        consistency: None,
        deadline: None,
        client_kind: VidaClientKind::Service,
        project_ref: None,
        claim_kind: None,
        trusted_owned_path: None,
        trusted_owned_write_scopes: Vec::new(),
        payload: serde_json::json!({"scenario":"runtime-engine-conformance"}),
        correlation: Some(serde_json::json!({"trace_id":"conformance-trace"})),
        idempotency_key: Some(VidaIdempotencyKey("conformance-idempotency".to_string())),
        apply_token: None,
    }
}
