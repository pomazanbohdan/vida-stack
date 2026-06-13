use std::path::PathBuf;

use runtime_path_policy::{ArtifactPathKind, StateRoot, existing_regular_file_under_root};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::HostBridgeError;

const MAX_HOST_BRIDGE_REQUEST_BYTES: u64 = 1024 * 1024;

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
            owned_paths: path_array(&raw, "owned_paths"),
            raw,
        })
    }
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
    }
}
