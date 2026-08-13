use std::path::{Path, PathBuf};

use runtime_path_policy::{ArtifactPathKind, StateRoot, existing_regular_file_under_root};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adapter_contract::{HostBridgeAdapterContractError, HostBridgeAdapterOperations};
use crate::errors::HostBridgeError;

const MAX_HOST_BRIDGE_REQUEST_BYTES: u64 = 1024 * 1024;
pub const HOST_BRIDGE_REQUIRED_RESULT_FIELDS: &[&str] = &[
    "decision",
    "verdict",
    "blocker_codes",
    "rework_target",
    "allowed_next_node",
];

pub const HOST_BRIDGE_REQUIRED_IDENTITY_FIELDS: &[&str] = &[
    "request_id",
    "run_id",
    "task_id",
    "attempt_id",
    "packet_id",
    "packet_path",
    "result_path",
    "receipt_path",
    "dispatch_target",
    "backend_id",
    "carrier_id",
    "adapter_kind",
    "adapter_capability_id",
    "invocation_mode",
    "dispatch_transport",
];

pub const HOST_BRIDGE_REQUIRED_CURRENT_SCHEMA_FIELDS: &[&str] = &[
    "schema_version",
    "execution_boundary",
    "receipt_mode",
    "adapter_contract_source",
    "adapter_contract_snapshot",
    "adapter_contract_hash",
    "request_path",
];

pub fn validate_host_bridge_request_identity(
    value: &Value,
) -> Result<HostBridgeAdapterOperations, HostBridgeError> {
    match value.get("schema_version") {
        None => {
            return Err(HostBridgeError::MissingRequiredField {
                field: "schema_version",
            });
        }
        Some(version) if version.as_u64() != Some(1) => {
            return Err(HostBridgeError::InvalidRequiredField {
                field: "schema_version",
            });
        }
        Some(_) => {}
    }
    for field in HOST_BRIDGE_REQUIRED_IDENTITY_FIELDS {
        required_string(value, field)?;
    }
    for field in [
        "execution_boundary",
        "receipt_mode",
        "adapter_contract_source",
        "adapter_contract_hash",
        "request_path",
    ] {
        required_string(value, field)?;
    }
    if value.get("adapter_operations").is_none() {
        return Err(HostBridgeError::AdapterContract(
            HostBridgeAdapterContractError::MissingField("adapter_operations"),
        ));
    }
    let contract = HostBridgeAdapterOperations::from_request_value(value)
        .map_err(HostBridgeError::AdapterContract)?;
    for field in [
        "adapter_kind",
        "adapter_capability_id",
        "invocation_mode",
        "dispatch_transport",
        "receipt_mode",
    ] {
        let top_level = required_string(value, field)?;
        let nested = match field {
            "adapter_kind" => &contract.adapter_kind,
            "adapter_capability_id" => &contract.adapter_capability_id,
            "invocation_mode" => &contract.invocation_mode,
            "dispatch_transport" => &contract.dispatch_transport,
            "receipt_mode" => &contract.receipt_mode,
            _ => unreachable!("parity fields are exhaustive"),
        };
        if top_level != *nested {
            return Err(HostBridgeError::InvalidRequiredField { field });
        }
    }
    let snapshot = value
        .get("adapter_contract_snapshot")
        .filter(|snapshot| snapshot.is_object())
        .ok_or(HostBridgeError::InvalidRequiredField {
            field: "adapter_contract_snapshot",
        })?;
    let canonical_snapshot = contract.to_value();
    if snapshot != &canonical_snapshot {
        return Err(HostBridgeError::InvalidRequiredField {
            field: "adapter_contract_snapshot",
        });
    }
    let expected_hash = blake3::hash(&serde_json::to_vec(snapshot).map_err(|_| {
        HostBridgeError::InvalidRequiredField {
            field: "adapter_contract_snapshot",
        }
    })?)
    .to_hex()
    .to_string();
    if required_string(value, "adapter_contract_hash")? != expected_hash {
        return Err(HostBridgeError::InvalidRequiredField {
            field: "adapter_contract_hash",
        });
    }
    Ok(contract)
}

pub fn host_bridge_request_error_fields(error: &HostBridgeError) -> Vec<String> {
    match error {
        HostBridgeError::MissingRequiredField { field } => vec![(*field).to_string()],
        HostBridgeError::InvalidRequiredField { field }
            if matches!(
                *field,
                "adapter_kind"
                    | "adapter_capability_id"
                    | "invocation_mode"
                    | "dispatch_transport"
                    | "receipt_mode"
            ) =>
        {
            vec!["adapter_operations".to_string()]
        }
        HostBridgeError::InvalidRequiredField { field } => vec![(*field).to_string()],
        HostBridgeError::AdapterContract(_) => vec!["adapter_operations".to_string()],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostBridgeRequestPath {
    pub state_root: PathBuf,
    pub request_path: PathBuf,
}

impl HostBridgeRequestPath {
    #[must_use]
    pub fn new(state_root: impl Into<PathBuf>, request_path: impl Into<PathBuf>) -> Self {
        Self {
            state_root: state_root.into(),
            request_path: request_path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBridgeRequest {
    pub schema_version: u32,
    pub status: String,
    pub request_id: String,
    pub run_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub packet_id: String,
    pub dispatch_target: String,
    pub packet_path: PathBuf,
    pub backend_id: String,
    pub carrier_id: String,
    pub execution_boundary: String,
    pub dispatch_transport: String,
    pub receipt_mode: String,
    pub adapter_kind: String,
    pub adapter_capability_id: String,
    pub invocation_mode: String,
    pub adapter_contract_source: String,
    pub adapter_contract_snapshot: Value,
    pub adapter_contract_hash: String,
    pub adapter_operations: Option<HostBridgeAdapterOperations>,
    pub request_path: PathBuf,
    pub result_path: PathBuf,
    pub receipt_path: PathBuf,
    pub required_proof_outputs: Vec<String>,
    pub required_result_fields: Vec<String>,
    pub owned_paths: Vec<PathBuf>,
    pub raw: Value,
}

impl HostBridgeRequest {
    pub fn from_value(raw: Value) -> Result<Self, HostBridgeError> {
        let adapter_operations = validate_host_bridge_request_identity(&raw)?;
        Ok(Self {
            schema_version: 1,
            status: required_string(&raw, "status")?,
            request_id: required_string(&raw, "request_id")?,
            run_id: required_string(&raw, "run_id")?,
            task_id: required_string(&raw, "task_id")?,
            attempt_id: required_string(&raw, "attempt_id")?,
            packet_id: required_string(&raw, "packet_id")?,
            dispatch_target: required_string(&raw, "dispatch_target")?,
            packet_path: required_path(&raw, "packet_path")?,
            backend_id: required_string(&raw, "backend_id")?,
            carrier_id: required_string(&raw, "carrier_id")?,
            execution_boundary: required_string(&raw, "execution_boundary")?,
            dispatch_transport: required_string(&raw, "dispatch_transport")?,
            receipt_mode: required_string(&raw, "receipt_mode")?,
            adapter_kind: required_string(&raw, "adapter_kind")?,
            adapter_capability_id: required_string(&raw, "adapter_capability_id")?,
            invocation_mode: required_string(&raw, "invocation_mode")?,
            adapter_contract_source: required_string(&raw, "adapter_contract_source")?,
            adapter_contract_snapshot: raw.get("adapter_contract_snapshot").cloned().ok_or(
                HostBridgeError::MissingRequiredField {
                    field: "adapter_contract_snapshot",
                },
            )?,
            adapter_contract_hash: required_string(&raw, "adapter_contract_hash")?,
            adapter_operations: Some(adapter_operations),
            request_path: required_path(&raw, "request_path")?,
            result_path: required_path(&raw, "result_path")?,
            receipt_path: required_path(&raw, "receipt_path")?,
            required_proof_outputs: host_bridge_required_proof_outputs(&raw),
            required_result_fields: host_bridge_required_result_fields(&raw),
            owned_paths: path_array(&raw, "owned_paths"),
            raw,
        })
    }
}

#[must_use]
pub fn default_host_bridge_required_result_fields() -> Vec<String> {
    HOST_BRIDGE_REQUIRED_RESULT_FIELDS
        .iter()
        .map(|field| (*field).to_string())
        .collect()
}

#[must_use]
pub fn host_bridge_required_result_fields(request: &Value) -> Vec<String> {
    let mut fields = string_array(request, "required_result_fields");
    if !host_bridge_required_proof_outputs(request).is_empty() {
        fields.extend(["proof_outputs".to_string(), "artifact_refs".to_string()]);
    }
    canonical_host_bridge_required_result_fields(fields)
}

#[must_use]
pub fn host_bridge_required_proof_outputs(request: &Value) -> Vec<String> {
    string_array(request, "required_proof_outputs")
}

#[must_use]
pub fn canonical_host_bridge_required_result_fields(
    request_fields: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut fields = default_host_bridge_required_result_fields();
    for field in request_fields {
        let field = field.trim();
        if !field.is_empty() && !fields.iter().any(|required| required == field) {
            fields.push(field.to_string());
        }
    }
    fields
}

pub fn host_bridge_request_string<'a>(request: &'a Value, field: &str) -> Option<&'a str> {
    request
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn host_bridge_request_task_class(request: &Value) -> Option<&str> {
    host_bridge_request_string(request, "task_class")
        .or_else(|| find_host_bridge_request_string(request, &["handoff_task_class", "task_class"]))
}

fn find_host_bridge_request_string<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a str> {
    let object = value.as_object()?;
    for field in fields {
        if let Some(value) = host_bridge_request_string(value, field) {
            return Some(value);
        }
    }
    object
        .values()
        .find_map(|value| find_host_bridge_request_string(value, fields))
}

pub fn host_bridge_blocked_result_contract(
    request: &Value,
) -> Option<&serde_json::Map<String, Value>> {
    let object = request.as_object()?;
    if let Some(contract) = object
        .get("blocked_result_contract")
        .and_then(Value::as_object)
    {
        return Some(contract);
    }
    object
        .get("host_bridge")
        .and_then(|host_bridge| host_bridge.get("blocked_result_contract"))
        .and_then(Value::as_object)
}

pub fn host_bridge_blocked_result_contract_allowed_next_node(request: &Value) -> Option<&str> {
    host_bridge_blocked_result_contract(request)
        .and_then(|contract| contract.get("allowed_next_node"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "next")
}

pub fn host_bridge_blocked_result_contract_is_retryable(
    contract: &serde_json::Map<String, Value>,
) -> bool {
    let has_retryable_route = contract
        .get("allowed_next_node")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "next");
    has_retryable_route
        && host_bridge_blocked_result_contract_field_is_retryable(contract, "decision")
        && host_bridge_blocked_result_contract_field_is_retryable(contract, "verdict")
}

pub fn host_bridge_blocked_result_contract_has_retry_evidence(
    _contract: &serde_json::Map<String, Value>,
) -> bool {
    // A blocked_result_contract is request metadata and may be project-controlled.
    // It can describe the shape of an acceptable blocked result, but it is not
    // independent execution evidence. Retry evidence must come from a persisted
    // host-bridge result or receipt artifact validated by the caller.
    false
}

fn host_bridge_blocked_result_contract_field_is_retryable(
    contract: &serde_json::Map<String, Value>,
    field: &str,
) -> bool {
    contract
        .get(field)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase().replace('-', "_"))
        .is_some_and(|value| {
            matches!(
                value.as_str(),
                "rework"
                    | "rework_required"
                    | "fail"
                    | "failed"
                    | "failure"
                    | "blocked"
                    | "retryable_blocked"
            )
        })
}

pub fn host_bridge_path_array(value: &Value, field: &str) -> Vec<PathBuf> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn proof_artifact_path_is_safe(path: &Path) -> bool {
    let normalized = path.display().to_string().replace('\\', "/");
    if normalized.is_empty()
        || normalized == "."
        || normalized == ".."
        || normalized == "/"
        || normalized.starts_with('/')
        || normalized.starts_with(".vida/")
        || normalized == ".vida"
        || normalized.starts_with("~/")
        || normalized.starts_with("//")
        || normalized.ends_with('/')
        || !normalized.contains('/')
    {
        return false;
    }
    if normalized.len() >= 2
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return false;
    }
    let components = normalized.split('/').collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.is_empty() || *component == "." || *component == "..")
    {
        return false;
    }
    proof_artifact_components_have_proof_context(&components)
}

fn proof_artifact_components_have_proof_context(components: &[&str]) -> bool {
    components.iter().any(|component| {
        matches!(
            *component,
            "test" | "tests" | "__tests__" | "spec" | "specs" | "proof" | "proofs"
        ) || component.ends_with("_test.rs")
            || component.ends_with("_test.dart")
            || component.ends_with(".test.ts")
            || component.ends_with(".test.tsx")
            || component.ends_with(".spec.ts")
            || component.ends_with(".spec.tsx")
    })
}

pub fn host_bridge_proof_artifact_path_array(value: &Value, field: &str) -> Vec<PathBuf> {
    host_bridge_path_array(value, field)
        .into_iter()
        .filter(|path| proof_artifact_path_is_safe(path))
        .collect()
}

pub fn host_bridge_request_owned_paths(request: &Value) -> Vec<PathBuf> {
    let mut owned_paths = host_bridge_path_array(request, "owned_paths");
    if owned_paths.is_empty()
        && let Some(implementation_isolation) = request.get("implementation_isolation")
    {
        owned_paths = host_bridge_path_array(implementation_isolation, "owned_paths");
    }
    owned_paths
}

pub fn host_bridge_request_proof_artifact_paths(request: &Value) -> Vec<PathBuf> {
    for field in [
        "proof_artifact_paths",
        "proof_artifact_scope",
        "proof_scope",
        "test_owned_paths",
        "proof_owned_paths",
    ] {
        let paths = host_bridge_proof_artifact_path_array(request, field);
        if !paths.is_empty() {
            return paths;
        }
    }
    if let Some(implementation_isolation) = request.get("implementation_isolation") {
        for field in [
            "proof_artifact_paths",
            "proof_artifact_scope",
            "proof_scope",
            "test_owned_paths",
            "proof_owned_paths",
        ] {
            let paths = host_bridge_proof_artifact_path_array(implementation_isolation, field);
            if !paths.is_empty() {
                return paths;
            }
        }
    }
    Vec::new()
}

pub fn legacy_internal_subagents_host_bridge_request(request: &Value) -> bool {
    if request.get("adapter_operations").is_some()
        || host_bridge_request_string(request, "backend_id") != Some("internal_subagents")
        || host_bridge_request_string(request, "dispatch_transport") != Some("host_tool_bridge")
        || host_bridge_request_string(request, "adapter_kind")
            != Some("unconfigured_host_agent_adapter")
        || host_bridge_request_string(request, "adapter_capability_id")
            != Some("unconfigured_host_agent_capability")
        || host_bridge_request_string(request, "invocation_mode")
            != Some("configured_host_capability_required")
    {
        return false;
    }
    let Some(adapter_params) = request.get("adapter_params").and_then(Value::as_object) else {
        return false;
    };
    ["spawn_tool", "wait_tool"].iter().all(|field| {
        adapter_params
            .get(*field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

pub fn effective_host_bridge_request(request: &Value) -> Value {
    request.clone()
}

/// Translate persisted legacy adapter aliases with an explicit registry row.
/// Unknown or absent mappings remain an error and must be surfaced as a typed
/// capability blocker by the caller.
pub fn effective_host_bridge_request_with_registry(
    request: &Value,
    registry: &Value,
) -> Result<Value, HostBridgeAdapterContractError> {
    let contract = HostBridgeAdapterOperations::from_registry_value(registry)?;
    let mut effective = request.clone();
    let object = effective
        .as_object_mut()
        .ok_or(HostBridgeAdapterContractError::InvalidField("request"))?;
    object.insert(
        "adapter_kind".to_string(),
        Value::String(contract.adapter_kind.clone()),
    );
    object.insert(
        "adapter_capability_id".to_string(),
        Value::String(contract.adapter_capability_id.clone()),
    );
    object.insert(
        "invocation_mode".to_string(),
        Value::String(contract.invocation_mode.clone()),
    );
    object.insert(
        "dispatch_transport".to_string(),
        Value::String(contract.dispatch_transport.clone()),
    );
    object.insert(
        "receipt_mode".to_string(),
        Value::String(contract.receipt_mode.clone()),
    );
    object.insert("adapter_operations".to_string(), contract.to_value());
    object.insert(
        "adapter_contract_source".to_string(),
        Value::String("configured_registry".to_string()),
    );
    Ok(effective)
}

pub fn read_host_bridge_request(
    path: &HostBridgeRequestPath,
) -> Result<HostBridgeRequest, HostBridgeError> {
    let state_root = StateRoot::open(&path.state_root)?;
    let request_path = existing_regular_file_under_root(
        &state_root,
        &path.request_path,
        ArtifactPathKind::HostBridgeRequest,
    )?;
    let metadata =
        std::fs::metadata(request_path.path()).map_err(|source| HostBridgeError::Read {
            path: request_path.path().to_path_buf(),
            source,
        })?;
    if metadata.len() > MAX_HOST_BRIDGE_REQUEST_BYTES {
        return Err(HostBridgeError::Oversized {
            path: request_path.path().to_path_buf(),
            max_bytes: MAX_HOST_BRIDGE_REQUEST_BYTES,
        });
    }
    let contents =
        std::fs::read_to_string(request_path.path()).map_err(|source| HostBridgeError::Read {
            path: request_path.path().to_path_buf(),
            source,
        })?;
    let mut raw: Value =
        serde_json::from_str(&contents).map_err(|source| HostBridgeError::Json {
            path: request_path.path().to_path_buf(),
            source,
        })?;
    enrich_request_paths(&mut raw, request_path.path());

    HostBridgeRequest::from_value(raw)
}

fn required_string(value: &Value, field: &'static str) -> Result<String, HostBridgeError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or(HostBridgeError::MissingRequiredField { field })
}

fn required_path(value: &Value, field: &'static str) -> Result<PathBuf, HostBridgeError> {
    Ok(PathBuf::from(required_string(value, field)?))
}

fn path_array(value: &Value, field: &'static str) -> Vec<PathBuf> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .collect()
}

fn string_array(value: &Value, field: &'static str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn enrich_request_paths(raw: &mut Value, request_path: &std::path::Path) {
    let Some(object) = raw.as_object_mut() else {
        return;
    };
    object
        .entry("request_path")
        .or_insert_with(|| Value::String(request_path.display().to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_current_request() -> Value {
        let mut request = serde_json::json!({
            "schema_version": 1,
            "status": "pending",
            "request_id": "req-1",
            "run_id": "run-1",
            "task_id": "task-1",
            "attempt_id": "attempt-1",
            "packet_id": "packet-1",
            "packet_path": "packet.json",
            "result_path": "result.json",
            "receipt_path": "receipt.json",
            "dispatch_target": "implementer",
            "backend_id": "internal_subagents",
            "carrier_id": "middle",
            "execution_boundary": "parent_host_session",
            "dispatch_transport": "host_tool_bridge",
            "receipt_mode": "host_bridge_receipt",
            "adapter_kind": "codex_host_tools",
            "adapter_capability_id": "codex.multi_agent_v1",
            "invocation_mode": "parent_host_tool_api",
            "adapter_contract_source": "configured_registry",
            "adapter_operations": {
                "adapter_kind": "codex_host_tools",
                "adapter_capability_id": "codex.multi_agent_v1",
                "invocation_mode": "parent_host_tool_api",
                "dispatch_transport": "host_tool_bridge",
                "receipt_mode": "host_bridge_receipt",
                "operations": {
                    "spawn": "multi_agent_v1.spawn_agent",
                    "wait": "multi_agent_v1.wait_agent",
                    "dispose": "multi_agent_v1.close_agent"
                },
                "dispose_policy": "configured"
            },
            "request_path": "request.json"
        });
        let snapshot = request["adapter_operations"].clone();
        let hash = blake3::hash(&serde_json::to_vec(&snapshot).expect("snapshot serializes"))
            .to_hex()
            .to_string();
        request["adapter_contract_snapshot"] = snapshot;
        request["adapter_contract_hash"] = Value::String(hash);
        request
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "taskflow-host-bridge-request-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("host-tool-bridge").join("requests")).unwrap();
        root
    }

    #[test]
    fn request_string_trims_and_rejects_blank_values() {
        let request = serde_json::json!({
            "run_id": " run-1 ",
            "blank": "   "
        });

        assert_eq!(
            host_bridge_request_string(&request, "run_id"),
            Some("run-1")
        );
        assert_eq!(host_bridge_request_string(&request, "blank"), None);
    }

    #[test]
    fn request_task_class_prefers_direct_non_blank_value() {
        let request = serde_json::json!({
            "task_class": " implementation ",
            "delivery_task_packet": {"handoff_task_class": "nested"}
        });

        assert_eq!(
            host_bridge_request_task_class(&request),
            Some("implementation")
        );
    }

    #[test]
    fn request_owned_paths_falls_back_to_implementation_isolation() {
        let request = serde_json::json!({
            "implementation_isolation": {
                "owned_paths": ["crates/vida", " "]
            }
        });

        assert_eq!(
            host_bridge_request_owned_paths(&request),
            vec![PathBuf::from("crates/vida")]
        );
    }

    #[test]
    fn request_owned_paths_prefers_direct_paths_over_nested_isolation() {
        let request = serde_json::json!({
            "owned_paths": ["crates/direct"],
            "implementation_isolation": {"owned_paths": ["crates/nested"]}
        });

        assert_eq!(
            host_bridge_request_owned_paths(&request),
            vec![PathBuf::from("crates/direct")]
        );
    }

    #[test]
    fn request_proof_artifact_paths_support_explicit_request_scope() {
        let request = serde_json::json!({
            "proof_artifact_scope": ["src/test/features/list_view", " "]
        });

        assert_eq!(
            host_bridge_request_proof_artifact_paths(&request),
            vec![PathBuf::from("src/test/features/list_view")]
        );
    }

    #[test]
    fn request_proof_artifact_paths_support_request_paths_field() {
        let request = serde_json::json!({
            "proof_artifact_paths": ["src/test/features/list_view/domain/model_test.dart", " "]
        });

        assert_eq!(
            host_bridge_request_proof_artifact_paths(&request),
            vec![PathBuf::from(
                "src/test/features/list_view/domain/model_test.dart"
            )]
        );
    }

    #[test]
    fn request_proof_artifact_paths_reject_scope_expanding_paths() {
        let request = serde_json::json!({
            "proof_artifact_paths": [
                "../..",
                "/",
                "/etc/passwd",
                ".vida/data/state",
                "C:/Windows",
                "crates/vida",
                "tests/",
                "src/test/features/list_view/domain/model_test.dart"
            ]
        });

        assert_eq!(
            host_bridge_request_proof_artifact_paths(&request),
            vec![PathBuf::from(
                "src/test/features/list_view/domain/model_test.dart"
            )]
        );
    }

    #[test]
    fn request_proof_artifact_paths_fall_back_to_isolation_scope() {
        let request = serde_json::json!({
            "implementation_isolation": {
                "proof_scope": ["tests/record_chatter"]
            }
        });

        assert_eq!(
            host_bridge_request_proof_artifact_paths(&request),
            vec![PathBuf::from("tests/record_chatter")]
        );
    }

    #[test]
    fn effective_request_materializes_legacy_internal_subagent_adapter_defaults() {
        let request = serde_json::json!({
            "backend_id": "internal_subagents",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "unconfigured_host_agent_adapter"
        });

        let registry = serde_json::json!({
            "adapter_kind": "configured_adapter",
            "adapter_capability_id": "configured_capability",
            "invocation_mode": "configured_parent",
            "dispatch_transport": "configured_transport",
            "receipt_mode": "configured_receipt",
            "operations": {
                "spawn": "configured.spawn",
                "wait": "configured.wait",
                "dispose": "configured.dispose"
            },
            "dispose_policy": "configured"
        });
        let effective = effective_host_bridge_request_with_registry(&request, &registry).unwrap();

        assert_eq!(effective["adapter_kind"], "configured_adapter");
        assert_eq!(effective["adapter_capability_id"], "configured_capability");
        assert_eq!(effective["invocation_mode"], "configured_parent");
        assert_eq!(effective["dispatch_transport"], "configured_transport");
        assert_eq!(effective["receipt_mode"], "configured_receipt");
        assert_eq!(effective["adapter_contract_source"], "configured_registry");
        assert_eq!(
            effective["adapter_operations"]["operations"]["spawn"],
            "configured.spawn"
        );
        assert_eq!(
            effective["adapter_operations"]["operations"]["wait"],
            "configured.wait"
        );
        assert_eq!(
            effective["adapter_operations"]["operations"]["dispose"],
            "configured.dispose"
        );
        assert_eq!(effective["adapter_operations"]["dispose_policy"], "configured");
    }

    #[test]
    fn blocked_result_contract_helpers_find_nested_explicit_next_node() {
        let request = serde_json::json!({
            "status": "blocked",
            "host_bridge": {
                "blocked_result_contract": {
                    "decision": "rework_required",
                    "verdict": "rework_required",
                    "allowed_next_node": "alpha_rework"
                }
            }
        });

        let contract = host_bridge_blocked_result_contract(&request)
            .expect("nested blocked result contract should be found");

        assert_eq!(
            contract.get("allowed_next_node").and_then(Value::as_str),
            Some("alpha_rework")
        );
        assert_eq!(
            host_bridge_blocked_result_contract_allowed_next_node(&request),
            Some("alpha_rework")
        );
        assert!(host_bridge_blocked_result_contract_is_retryable(contract));
    }

    #[test]
    fn blocked_result_contract_retryability_requires_each_independent_signal() {
        let cases = [
            (serde_json::json!({
                "allowed_next_node": "alpha_rework",
                "decision": "rework_required",
                "verdict": "rework_required"
            }), true),
            (serde_json::json!({
                "allowed_next_node": "alpha_rework",
                "decision": "approved",
                "verdict": "rework_required"
            }), false),
            (serde_json::json!({
                "allowed_next_node": "alpha_rework",
                "decision": "rework_required",
                "verdict": "approved"
            }), false),
            (serde_json::json!({
                "allowed_next_node": "next",
                "decision": "rework_required",
                "verdict": "rework_required"
            }), false),
        ];

        for (contract, expected) in cases {
            assert_eq!(
                host_bridge_blocked_result_contract_is_retryable(
                    contract.as_object().expect("contract object")
                ),
                expected,
                "unexpected retryability for {contract}"
            );
        }
    }

    #[test]
    fn blocked_result_contract_retry_evidence_rejects_request_local_metadata() {
        let complete = serde_json::json!({
            "decision": "rework_required",
            "verdict": "rework_required",
            "allowed_next_node": "alpha_rework",
            "rework_target": "alpha",
            "blocker_codes": ["host_agent_execution_failed"]
        });
        assert!(host_bridge_blocked_result_contract_is_retryable(
            complete.as_object().expect("contract object")
        ));
        assert!(
            !host_bridge_blocked_result_contract_has_retry_evidence(
                complete.as_object().expect("contract object")
            ),
            "request-local blocked_result_contract metadata is not independent retry evidence"
        );

        let incomplete = serde_json::json!({
            "decision": "rework_required",
            "verdict": "rework_required",
            "allowed_next_node": "alpha_rework"
        });
        assert!(!host_bridge_blocked_result_contract_has_retry_evidence(
            incomplete.as_object().expect("contract object")
        ));
    }

    #[test]
    fn blocked_result_contract_helpers_reject_incomplete_nested_metadata_contract() {
        let request = serde_json::json!({
            "status": "blocked",
            "metadata": {
                "blocked_result_contract": {
                    "decision": "rework_required",
                    "verdict": "rework_required",
                    "allowed_next_node": "alpha_rework"
                }
            },
            "host_bridge": {
                "blocked_result_contract": {
                    "allowed_next_node": "metadata_rework"
                }
            }
        });
        let contract = host_bridge_blocked_result_contract(&request)
            .expect("trusted host_bridge blocked result contract should be found");

        assert_eq!(
            contract.get("allowed_next_node").and_then(Value::as_str),
            Some("metadata_rework")
        );
        assert_eq!(
            host_bridge_blocked_result_contract_allowed_next_node(&request),
            Some("metadata_rework")
        );
        assert!(!host_bridge_blocked_result_contract_is_retryable(contract));
    }

    #[test]
    fn blocked_result_contract_helpers_reject_abstract_next_node() {
        let request = serde_json::json!({
            "blocked_result_contract": {
                "allowed_next_node": "next"
            }
        });
        let contract = host_bridge_blocked_result_contract(&request)
            .expect("blocked result contract should be found");

        assert_eq!(
            host_bridge_blocked_result_contract_allowed_next_node(&request),
            None
        );
        assert!(!host_bridge_blocked_result_contract_is_retryable(contract));
    }

    #[test]
    fn request_task_class_falls_back_to_nested_handoff_class() {
        let request = serde_json::json!({
            "dispatch_target": "alpha_impl",
            "delivery_task_packet": {
                "handoff_task_class": " implementation "
            }
        });

        assert_eq!(
            host_bridge_request_task_class(&request),
            Some("implementation")
        );
    }

    #[test]
    fn read_request_requires_core_identity_fields() {
        let root = temp_root("missing-field");
        let request = root
            .join("host-tool-bridge")
            .join("requests")
            .join("request.json");
        std::fs::write(&request, r#"{"status":"pending"}"#).unwrap();

        let err =
            read_host_bridge_request(&HostBridgeRequestPath::new(&root, &request)).unwrap_err();

        assert!(matches!(
            err,
            HostBridgeError::MissingRequiredField {
                field: "schema_version"
            }
        ));
    }

    #[test]
    fn read_request_loads_minimal_envelope() {
        let root = temp_root("minimal-envelope");
        let request = root
            .join("host-tool-bridge")
            .join("requests")
            .join("request.json");
        let mut request_value = complete_current_request();
        request_value["dispatch_target"] = Value::String("developer".to_string());
        request_value["packet_path"] = Value::String("runtime-consumption/packet.json".to_string());
        request_value["request_path"] =
            Value::String("host-tool-bridge/requests/request.json".to_string());
        request_value["result_path"] =
            Value::String("host-tool-bridge/results/result.json".to_string());
        request_value["receipt_path"] =
            Value::String("host-tool-bridge/receipts/receipt.json".to_string());
        request_value["owned_paths"] =
            serde_json::json!(["crates/taskflow-host-bridge/src/lib.rs"]);
        std::fs::write(&request, request_value.to_string()).unwrap();

        let loaded =
            read_host_bridge_request(&HostBridgeRequestPath::new(&root, &request)).unwrap();

        assert_eq!(loaded.request_id, "req-1");
        assert_eq!(loaded.owned_paths.len(), 1);
        assert!(loaded.required_proof_outputs.is_empty());
        assert_eq!(loaded.invocation_mode, "parent_host_tool_api");
        assert_eq!(
            loaded.required_result_fields,
            default_host_bridge_required_result_fields()
        );
    }


    #[test]
    fn read_request_rejects_oversized_payload_with_exact_path() {
        let root = temp_root("oversized-payload");
        let request = root
            .join("host-tool-bridge")
            .join("requests")
            .join("request.json");
        let payload = vec![b'x'; (MAX_HOST_BRIDGE_REQUEST_BYTES + 1) as usize];
        std::fs::write(&request, payload).unwrap();

        let err =
            read_host_bridge_request(&HostBridgeRequestPath::new(&root, &request)).unwrap_err();

        assert!(
            matches!(
                &err,
                HostBridgeError::Oversized { path, max_bytes }
                    if path.ends_with(request.file_name().expect("request filename"))
                        && *max_bytes == MAX_HOST_BRIDGE_REQUEST_BYTES
            ),
            "unexpected oversized-payload error: {err:?}"
        );
    }


    #[test]
    fn read_request_accepts_exact_size_before_json_validation() {
        let root = temp_root("exact-size-payload");
        let request = root
            .join("host-tool-bridge")
            .join("requests")
            .join("request.json");
        let payload = vec![b'x'; MAX_HOST_BRIDGE_REQUEST_BYTES as usize];
        std::fs::write(&request, payload).unwrap();

        let err =
            read_host_bridge_request(&HostBridgeRequestPath::new(&root, &request)).unwrap_err();

        assert!(matches!(err, HostBridgeError::Json { .. }));
    }

    #[test]
    fn request_required_result_fields_cannot_downgrade_canonical_contract() {
        let request = serde_json::json!({
            "required_result_fields": ["allowed_next_node", "custom_evidence"]
        });

        let fields = host_bridge_required_result_fields(&request);

        assert_eq!(
            &fields[..HOST_BRIDGE_REQUIRED_RESULT_FIELDS.len()],
            default_host_bridge_required_result_fields().as_slice()
        );
        assert!(fields.iter().any(|field| field == "custom_evidence"));
    }

    #[test]
    fn request_with_proof_requirements_requires_structured_proof_arrays() {
        let request = serde_json::json!({
            "required_proof_outputs": ["changed_files", "verification_notes"],
            "required_result_fields": ["decision"]
        });

        assert_eq!(
            host_bridge_required_proof_outputs(&request),
            ["changed_files", "verification_notes"]
        );
        let fields = host_bridge_required_result_fields(&request);
        assert!(fields.iter().any(|field| field == "proof_outputs"));
        assert!(fields.iter().any(|field| field == "artifact_refs"));
    }

    #[test]
    fn current_request_identity_requires_every_field_and_rejects_empty_values() {
        for field in HOST_BRIDGE_REQUIRED_IDENTITY_FIELDS {
            let mut missing = complete_current_request();
            missing.as_object_mut().unwrap().remove(*field);
            let error = validate_host_bridge_request_identity(&missing).unwrap_err();
            assert!(
                matches!(error, HostBridgeError::MissingRequiredField { field: actual } if actual == *field)
            );

            let mut empty = complete_current_request();
            empty[*field] = Value::String("  ".to_string());
            let error = validate_host_bridge_request_identity(&empty).unwrap_err();
            assert!(
                matches!(error, HostBridgeError::MissingRequiredField { field: actual } if actual == *field)
            );
        }
    }

    #[test]
    fn current_request_identity_accepts_valid_contract_and_rejects_parity_drift() {
        let request = complete_current_request();
        assert!(validate_host_bridge_request_identity(&request).is_ok());

        for field in [
            "adapter_kind",
            "adapter_capability_id",
            "invocation_mode",
            "dispatch_transport",
        ] {
            let mut drifted = request.clone();
            drifted[field] = Value::String("different_identity".to_string());
            let error = validate_host_bridge_request_identity(&drifted).unwrap_err();
            match error {
                HostBridgeError::InvalidRequiredField { field: actual }
                | HostBridgeError::AdapterContract(HostBridgeAdapterContractError::InvalidField(
                    actual,
                )) => assert_eq!(actual, field),
                other => panic!("unexpected parity error: {other:?}"),
            }
        }
    }

    #[test]
    fn legacy_ingress_is_explicit_and_does_not_classify_malformed_or_non_internal_rows() {
        let mut legacy = complete_current_request();
        let object = legacy.as_object_mut().unwrap();
        object.remove("adapter_operations");
        object.insert(
            "adapter_kind".to_string(),
            Value::String("unconfigured_host_agent_adapter".to_string()),
        );
        object.insert(
            "adapter_capability_id".to_string(),
            Value::String("unconfigured_host_agent_capability".to_string()),
        );
        object.insert(
            "invocation_mode".to_string(),
            Value::String("configured_host_capability_required".to_string()),
        );
        object.insert(
            "adapter_params".to_string(),
            serde_json::json!({
                "spawn_tool": "legacy.spawn",
                "wait_tool": "legacy.wait"
            }),
        );
        assert!(legacy_internal_subagents_host_bridge_request(&legacy));
        assert!(matches!(
            validate_host_bridge_request_identity(&legacy),
            Err(HostBridgeError::AdapterContract(
                HostBridgeAdapterContractError::MissingField("adapter_operations")
            ))
        ));

        let mut malformed = legacy.clone();
        malformed["adapter_params"] = serde_json::json!({ "spawn_tool": "legacy.spawn" });
        assert!(!legacy_internal_subagents_host_bridge_request(&malformed));

        let mut non_internal = legacy;
        non_internal["backend_id"] = Value::String("external_backend".to_string());
        assert!(!legacy_internal_subagents_host_bridge_request(
            &non_internal
        ));
    }


    #[test]
    fn legacy_ingress_requires_both_non_blank_adapter_tools() {
        let mut base = complete_current_request();
        let object = base.as_object_mut().unwrap();
        object.remove("adapter_operations");
        object.insert(
            "adapter_kind".to_string(),
            Value::String("unconfigured_host_agent_adapter".to_string()),
        );
        object.insert(
            "adapter_capability_id".to_string(),
            Value::String("unconfigured_host_agent_capability".to_string()),
        );
        object.insert(
            "invocation_mode".to_string(),
            Value::String("configured_host_capability_required".to_string()),
        );
        object.insert(
            "adapter_params".to_string(),
            serde_json::json!({"spawn_tool": "legacy.spawn", "wait_tool": "legacy.wait"}),
        );
        assert!(legacy_internal_subagents_host_bridge_request(&base));

        let mut operation_present = base.clone();
        operation_present["adapter_operations"] = serde_json::json!({});
        assert!(!legacy_internal_subagents_host_bridge_request(&operation_present));

        let mut missing_params = base.clone();
        missing_params
            .as_object_mut()
            .expect("request object")
            .remove("adapter_params");
        assert!(!legacy_internal_subagents_host_bridge_request(&missing_params));

        for params in [
            serde_json::json!({"wait_tool": "legacy.wait"}),
            serde_json::json!({"spawn_tool": "legacy.spawn"}),
            serde_json::json!({"spawn_tool": " ", "wait_tool": "legacy.wait"}),
            serde_json::json!({"spawn_tool": "legacy.spawn", "wait_tool": " "}),
        ] {
            let mut candidate = base.clone();
            candidate["adapter_params"] = params;
            assert!(!legacy_internal_subagents_host_bridge_request(&candidate));
        }
    }
}
