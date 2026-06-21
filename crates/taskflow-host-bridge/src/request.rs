use std::path::PathBuf;

use runtime_path_policy::{ArtifactPathKind, StateRoot, existing_regular_file_under_root};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
            backend_id: optional_string(&raw, "backend_id")
                .unwrap_or_else(|| "internal_subagents".to_string()),
            carrier_id: optional_string(&raw, "carrier_id")
                .unwrap_or_else(|| "internal_subagents".to_string()),
            execution_boundary: optional_string(&raw, "execution_boundary")
                .unwrap_or_else(|| "parent_host_session".to_string()),
            dispatch_transport: optional_string(&raw, "dispatch_transport")
                .unwrap_or_else(|| "host_tool_bridge".to_string()),
            receipt_mode: optional_string(&raw, "receipt_mode")
                .unwrap_or_else(|| "host_bridge_receipt".to_string()),
            adapter_kind: optional_string(&raw, "adapter_kind")
                .unwrap_or_else(|| "codex_host_tools".to_string()),
            adapter_capability_id: optional_string(&raw, "adapter_capability_id")
                .unwrap_or_else(|| "codex.multi_agent_v1".to_string()),
            invocation_mode: optional_string(&raw, "invocation_mode")
                .unwrap_or_else(|| "parent_host_tool_api".to_string()),
            adapter_contract_source: optional_string(&raw, "adapter_contract_source")
                .unwrap_or_else(|| "request".to_string()),
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

pub fn host_bridge_request_owned_paths(request: &Value) -> Vec<PathBuf> {
    let mut owned_paths = host_bridge_path_array(request, "owned_paths");
    if owned_paths.is_empty() {
        if let Some(implementation_isolation) = request.get("implementation_isolation") {
            owned_paths = host_bridge_path_array(implementation_isolation, "owned_paths");
        }
    }
    owned_paths
}

pub fn legacy_internal_subagents_host_bridge_request(request: &Value) -> bool {
    host_bridge_request_string(request, "backend_id") == Some("internal_subagents")
        && host_bridge_request_string(request, "dispatch_transport") == Some("host_tool_bridge")
        && (host_bridge_request_string(request, "adapter_kind")
            == Some("unconfigured_host_agent_adapter")
            || host_bridge_request_string(request, "adapter_capability_id")
                == Some("unconfigured_host_agent_capability")
            || host_bridge_request_string(request, "invocation_mode")
                == Some("configured_host_capability_required"))
}

pub fn effective_host_bridge_request(request: &Value) -> Value {
    if !legacy_internal_subagents_host_bridge_request(request) {
        return request.clone();
    }
    let mut effective = request.clone();
    if let Some(object) = effective.as_object_mut() {
        object.insert(
            "adapter_kind".to_string(),
            Value::String("codex_host_tools".to_string()),
        );
        object.insert(
            "adapter_capability_id".to_string(),
            Value::String("codex.multi_agent_v1".to_string()),
        );
        object.insert(
            "invocation_mode".to_string(),
            Value::String("parent_host_tool_api".to_string()),
        );
        object
            .entry("receipt_mode".to_string())
            .or_insert_with(|| Value::String("host_bridge_receipt".to_string()));
        object.insert(
            "adapter_contract_source".to_string(),
            Value::String("legacy_internal_subagents_default".to_string()),
        );
        let adapter_params = object
            .entry("adapter_params".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(params) = adapter_params.as_object_mut() {
            params.insert(
                "tool_family".to_string(),
                Value::String("codex_multi_agent".to_string()),
            );
            params.insert(
                "spawn_tool".to_string(),
                Value::String("multi_agent_v1.spawn_agent".to_string()),
            );
            params.insert(
                "wait_tool".to_string(),
                Value::String("multi_agent_v1.wait_agent".to_string()),
            );
            params.insert(
                "close_tool".to_string(),
                Value::String("multi_agent_v1.close_agent".to_string()),
            );
        }
    }
    effective
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
    fn effective_request_materializes_legacy_internal_subagent_adapter_defaults() {
        let request = serde_json::json!({
            "backend_id": "internal_subagents",
            "dispatch_transport": "host_tool_bridge",
            "adapter_kind": "unconfigured_host_agent_adapter"
        });

        let effective = effective_host_bridge_request(&request);

        assert_eq!(effective["adapter_kind"], "codex_host_tools");
        assert_eq!(effective["adapter_capability_id"], "codex.multi_agent_v1");
        assert_eq!(effective["invocation_mode"], "parent_host_tool_api");
        assert_eq!(
            effective["adapter_params"]["spawn_tool"],
            "multi_agent_v1.spawn_agent"
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
