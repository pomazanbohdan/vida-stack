use std::path::{Path, PathBuf};

use runtime_path_policy::{existing_regular_file_under_root, ArtifactPathKind, StateRoot};
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
    pub adapter_operations: Option<HostBridgeAdapterOperations>,
    pub request_path: PathBuf,
    pub result_path: PathBuf,
    pub receipt_path: PathBuf,
    pub required_result_fields: Vec<String>,
    pub owned_paths: Vec<PathBuf>,
    pub raw: Value,
}

impl HostBridgeRequest {
    pub fn from_value(raw: Value) -> Result<Self, HostBridgeError> {
        Ok(Self {
            schema_version: optional_u32(&raw, "schema_version").unwrap_or(0),
            status: required_string(&raw, "status")?,
            request_id: required_string(&raw, "request_id")?,
            run_id: required_string(&raw, "run_id")?,
            task_id: optional_string(&raw, "task_id")
                .unwrap_or_else(|| required_string(&raw, "run_id").unwrap_or_default()),
            dispatch_target: required_string(&raw, "dispatch_target")?,
            packet_path: optional_path(&raw, "packet_path").unwrap_or_default(),
            backend_id: optional_string(&raw, "backend_id").unwrap_or_default(),
            carrier_id: optional_string(&raw, "carrier_id").unwrap_or_default(),
            execution_boundary: optional_string(&raw, "execution_boundary").unwrap_or_default(),
            dispatch_transport: optional_string(&raw, "dispatch_transport").unwrap_or_default(),
            receipt_mode: optional_string(&raw, "receipt_mode").unwrap_or_default(),
            adapter_kind: optional_string(&raw, "adapter_kind").unwrap_or_default(),
            adapter_capability_id: optional_string(&raw, "adapter_capability_id")
                .unwrap_or_default(),
            invocation_mode: optional_string(&raw, "invocation_mode").unwrap_or_default(),
            adapter_contract_source: optional_string(&raw, "adapter_contract_source")
                .unwrap_or_default(),
            adapter_operations: HostBridgeAdapterOperations::from_request_value(&raw).ok(),
            request_path: optional_path(&raw, "request_path").unwrap_or_default(),
            result_path: optional_path(&raw, "result_path").unwrap_or_default(),
            receipt_path: optional_path(&raw, "receipt_path").unwrap_or_default(),
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
    canonical_host_bridge_required_result_fields(string_array(request, "required_result_fields"))
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
    if owned_paths.is_empty() {
        if let Some(implementation_isolation) = request.get("implementation_isolation") {
            owned_paths = host_bridge_path_array(implementation_isolation, "owned_paths");
        }
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
    HostBridgeAdapterOperations::from_request_value(request).is_err()
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

fn optional_path(value: &Value, field: &'static str) -> Option<PathBuf> {
    optional_string(value, field).map(PathBuf::from)
}

fn optional_string(value: &Value, field: &'static str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_u32(value: &Value, field: &'static str) -> Option<u32> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
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
        assert_eq!(
            effective["adapter_operations"]["operations"]["spawn"],
            "configured.spawn"
        );
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
                field: "request_id"
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
        std::fs::write(
            &request,
            serde_json::json!({
                "schema_version": 1,
                "status": "pending",
                "request_id": "req-1",
                "run_id": "run-1",
                "task_id": "task-1",
                "dispatch_target": "developer",
                "packet_path": "runtime-consumption/packet.json",
                "backend_id": "internal_subagents",
                "carrier_id": "senior",
                "execution_boundary": "parent_host_session",
                "dispatch_transport": "host_tool_bridge",
                "receipt_mode": "host_bridge_receipt",
                "adapter_kind": "codex_host_tools",
                "adapter_capability_id": "codex.multi_agent_v1",
                "invocation_mode": "parent_host_tool_api",
                "receipt_mode": "host_bridge_receipt",
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
                "request_path": "host-tool-bridge/requests/request.json",
                "result_path": "host-tool-bridge/results/result.json",
                "receipt_path": "host-tool-bridge/receipts/receipt.json",
                "required_result_fields": [
                    "decision",
                    "verdict",
                    "blocker_codes",
                    "rework_target",
                    "allowed_next_node"
                ],
                "owned_paths": ["crates/taskflow-host-bridge/src/lib.rs"]
            })
            .to_string(),
        )
        .unwrap();

        let loaded =
            read_host_bridge_request(&HostBridgeRequestPath::new(&root, &request)).unwrap();

        assert_eq!(loaded.request_id, "req-1");
        assert_eq!(loaded.owned_paths.len(), 1);
        assert_eq!(loaded.invocation_mode, "parent_host_tool_api");
        assert_eq!(
            loaded.required_result_fields,
            default_host_bridge_required_result_fields()
        );
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
}
